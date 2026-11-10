use std::io::Write;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::manifest::{self, Sha256Source, ToolSource};
use super::{manual_hint, Platform, ToolId};
use crate::error::{ErrorKind, UserError};

fn net(e: impl std::fmt::Display) -> UserError {
    UserError::new(ErrorKind::Network, e.to_string())
}

fn other(e: impl std::fmt::Display) -> UserError {
    UserError::new(ErrorKind::Other, e.to_string())
}

async fn get(client: &reqwest::Client, url: &str) -> Result<reqwest::Response, UserError> {
    let resp = client.get(url).send().await.map_err(net)?;
    if !resp.status().is_success() {
        return Err(net(format!("HTTP {} for {url}", resp.status())));
    }
    Ok(resp)
}

async fn expected_hash(client: &reqwest::Client, src: &Sha256Source) -> Result<String, UserError> {
    match src {
        Sha256Source::Fixed(h) => Ok(h.to_lowercase()),
        Sha256Source::SumsFile { url, name } => {
            let text = get(client, url).await?.text().await.map_err(net)?;
            text.lines()
                .find_map(|line| {
                    let mut parts = line.split_whitespace();
                    let hash = parts.next()?;
                    let file = parts.next()?.trim_start_matches('*');
                    (file == name).then(|| hash.to_lowercase())
                })
                .ok_or_else(|| other(format!("no checksum listed for {name}")))
        }
    }
}

/// Download `src`, verify its SHA-256, optionally gunzip it, mark it executable
/// and move it to `tools_dir/file_name`. Nothing is left behind on failure.
pub async fn install_from(
    src: &ToolSource,
    file_name: &str,
    tools_dir: &Path,
    on_progress: &(dyn Fn(u64, Option<u64>) + Send + Sync),
) -> Result<PathBuf, UserError> {
    let client = reqwest::Client::builder().user_agent("StreamFlow").build().map_err(net)?;
    let expected = expected_hash(&client, &src.sha256).await?;
    std::fs::create_dir_all(tools_dir).map_err(other)?;

    let mut resp = get(&client, &src.url).await?;
    let total = resp.content_length();
    let mut tmp = tempfile::NamedTempFile::new_in(tools_dir).map_err(other)?;
    let mut hasher = Sha256::new();
    let mut done = 0u64;
    while let Some(chunk) = resp.chunk().await.map_err(net)? {
        hasher.update(&chunk);
        tmp.write_all(&chunk).map_err(other)?;
        done += chunk.len() as u64;
        on_progress(done, total);
    }
    tmp.flush().map_err(other)?;
    if hex::encode(hasher.finalize()) != expected {
        return Err(other(format!("checksum mismatch for {file_name}")));
    }

    let ready = if src.gunzip {
        let dir = tools_dir.to_path_buf();
        let gz_path = tmp.path().to_path_buf();
        tokio::task::spawn_blocking(move || -> std::io::Result<tempfile::NamedTempFile> {
            let mut out = tempfile::NamedTempFile::new_in(dir)?;
            let mut dec = flate2::read::GzDecoder::new(std::fs::File::open(gz_path)?);
            std::io::copy(&mut dec, &mut out)?;
            out.flush()?;
            Ok(out)
        })
        .await
        .map_err(other)?
        .map_err(other)?
    } else {
        tmp
    };

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(ready.path(), std::fs::Permissions::from_mode(0o755)).map_err(other)?;
    }
    let dest = tools_dir.join(file_name);
    ready.persist(&dest).map_err(|e| other(e.error))?;
    Ok(dest)
}

/// Install (or update) `tool` for platform `p` into `tools_dir`.
pub async fn install(
    tool: ToolId,
    p: Platform,
    tools_dir: &Path,
    on_progress: &(dyn Fn(u64, Option<u64>) + Send + Sync),
) -> Result<PathBuf, UserError> {
    let src = manifest::source(tool, p)
        .ok_or_else(|| UserError::new(ErrorKind::ToolMissing, manual_hint(tool, p)))?;
    install_from(&src, tool.file_name(p.os), tools_dir, on_progress).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorKind;
    use crate::tools::manifest::{Sha256Source, ToolSource};
    use sha2::{Digest, Sha256};
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    /// Tiny HTTP server: serves fixed bodies by path, 404 otherwise.
    fn serve(routes: Vec<(&'static str, Vec<u8>)>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let routes: HashMap<&str, Vec<u8>> = routes.into_iter().collect();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut s) = stream else { continue };
                let mut buf = [0u8; 2048];
                let n = s.read(&mut buf).unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let path = req.split_whitespace().nth(1).unwrap_or("/").to_string();
                match routes.get(path.as_str()) {
                    Some(body) => {
                        let _ = write!(s, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                        let _ = s.write_all(body);
                    }
                    None => {
                        let _ = write!(s, "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                    }
                }
            }
        });
        base
    }

    fn sha(bytes: &[u8]) -> String {
        hex::encode(Sha256::digest(bytes))
    }

    fn noop(_: u64, _: Option<u64>) {}

    #[tokio::test]
    async fn install_rejects_bad_hash() {
        let base = serve(vec![("/tool", b"payload".to_vec())]);
        let dir = tempfile::tempdir().unwrap();
        let src = ToolSource { url: format!("{base}/tool"), sha256: Sha256Source::Fixed("0".repeat(64)), gunzip: false };
        let err = install_from(&src, "yt-dlp", dir.path(), &noop).await.unwrap_err();
        assert!(err.detail.to_lowercase().contains("checksum"), "{err:?}");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0, "nothing may be left behind");
    }

    #[tokio::test]
    async fn install_ok_sets_executable() {
        let base = serve(vec![("/tool", b"payload".to_vec())]);
        let dir = tempfile::tempdir().unwrap();
        let src = ToolSource { url: format!("{base}/tool"), sha256: Sha256Source::Fixed(sha(b"payload")), gunzip: false };
        let calls = std::sync::atomic::AtomicU64::new(0);
        let path = install_from(&src, "yt-dlp", dir.path(), &|d, _| calls.store(d, std::sync::atomic::Ordering::SeqCst)).await.unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"payload");
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 7);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_ne!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o111, 0);
        }
    }

    #[tokio::test]
    async fn install_gunzips() {
        let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        enc.write_all(b"ffmpeg-binary").unwrap();
        let gz = enc.finish().unwrap();
        let base = serve(vec![("/ffmpeg.gz", gz.clone())]);
        let dir = tempfile::tempdir().unwrap();
        let src = ToolSource { url: format!("{base}/ffmpeg.gz"), sha256: Sha256Source::Fixed(sha(&gz)), gunzip: true };
        let path = install_from(&src, "ffmpeg", dir.path(), &noop).await.unwrap();
        assert_eq!(std::fs::read(path).unwrap(), b"ffmpeg-binary");
    }

    #[tokio::test]
    async fn install_uses_sums_file() {
        let sums = format!("{}  other\n{}  yt-dlp_macos\n", "a".repeat(64), sha(b"payload"));
        let base = serve(vec![("/tool", b"payload".to_vec()), ("/sums", sums.into_bytes())]);
        let dir = tempfile::tempdir().unwrap();
        let src = ToolSource {
            url: format!("{base}/tool"),
            sha256: Sha256Source::SumsFile { url: format!("{base}/sums"), name: "yt-dlp_macos".into() },
            gunzip: false,
        };
        assert!(install_from(&src, "yt-dlp", dir.path(), &noop).await.is_ok());
        let missing = ToolSource { sha256: Sha256Source::SumsFile { url: format!("{base}/sums"), name: "nope".into() }, ..src };
        assert!(install_from(&missing, "yt-dlp", dir.path(), &noop).await.is_err());
    }

    #[tokio::test]
    async fn http_error_is_reported() {
        let base = serve(vec![]);
        let dir = tempfile::tempdir().unwrap();
        let src = ToolSource { url: format!("{base}/missing"), sha256: Sha256Source::Fixed("0".repeat(64)), gunzip: false };
        let err = install_from(&src, "x", dir.path(), &noop).await.unwrap_err();
        assert_eq!(err.kind, ErrorKind::Network);
    }
}
