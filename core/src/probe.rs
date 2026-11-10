use std::cmp::Ordering;
use std::path::Path;
use std::process::Stdio;

use serde_json::Value;

use crate::error::{ErrorKind, UserError};
use crate::model::{
    AudioChoice, AudioSelect, Format, VideoChoice, VideoInfo, VideoSelect,
};

static NULL: Value = Value::Null;

/// Trim the pasted text and make sure it looks like an http(s) link.
pub fn normalize_url(input: &str) -> Result<String, UserError> {
    let t = input.trim();
    let lower = t.to_ascii_lowercase();
    let rest = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))
        .unwrap_or("");
    if rest.is_empty() || t.chars().any(char::is_whitespace) {
        return Err(UserError::new(ErrorKind::InvalidUrl, t));
    }
    Ok(t.to_string())
}

fn str_field(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(str::to_owned)
}

fn num_field(v: &Value, key: &str) -> Option<f64> {
    v.get(key).and_then(Value::as_f64)
}

fn parse_format(v: &Value) -> Option<Format> {
    Some(Format {
        format_id: str_field(v, "format_id")?,
        ext: str_field(v, "ext").unwrap_or_default(),
        vcodec: str_field(v, "vcodec"),
        acodec: str_field(v, "acodec"),
        height: num_field(v, "height").map(|h| h as u32),
        tbr: num_field(v, "tbr"),
        abr: num_field(v, "abr"),
        filesize: num_field(v, "filesize").map(|n| n as u64),
        filesize_approx: num_field(v, "filesize_approx").map(|n| n as u64),
    })
}

/// Parse the output of `yt-dlp -J`. For a playlist the first entry is used.
pub fn parse_probe(json: &str) -> Result<VideoInfo, UserError> {
    let root: Value = serde_json::from_str(json)
        .map_err(|e| UserError::new(ErrorKind::Other, format!("invalid yt-dlp JSON: {e}")))?;
    let is_playlist = root.get("_type").and_then(Value::as_str) == Some("playlist");
    let entry = if is_playlist {
        root.get("entries")
            .and_then(Value::as_array)
            .and_then(|a| a.iter().find(|e| e.is_object()))
            .unwrap_or(&NULL)
    } else {
        &root
    };
    let pick = |key: &str| str_field(entry, key).or_else(|| str_field(&root, key));
    let formats = entry
        .get("formats")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(parse_format).collect())
        .unwrap_or_default();
    Ok(VideoInfo {
        id: pick("id").unwrap_or_default(),
        title: pick("title").unwrap_or_default(),
        uploader: pick("uploader"),
        duration_secs: num_field(entry, "duration").map(|d| d as u64),
        thumbnail: pick("thumbnail"),
        webpage_url: pick("webpage_url").unwrap_or_default(),
        is_playlist,
        formats,
    })
}

fn has_video(f: &Format) -> bool {
    match f.vcodec.as_deref() {
        Some(c) => c != "none",
        None => f.height.is_some(),
    }
}

fn has_audio(f: &Format) -> bool {
    matches!(f.acodec.as_deref(), Some(c) if c != "none")
}

fn size_of(f: &Format) -> Option<u64> {
    f.filesize.or(f.filesize_approx)
}

fn video_codec_label(vcodec: Option<&str>) -> String {
    match vcodec {
        Some(c) if c.starts_with("vp9") || c.starts_with("vp09") => "VP9".into(),
        Some(c) if c.starts_with("avc1") || c.starts_with("h264") => "H.264".into(),
        Some(c) if c.starts_with("av01") => "AV1".into(),
        Some(c) if c.starts_with("hev") || c.starts_with("hvc") => "H.265".into(),
        Some(c) if c != "none" => c.split('.').next().unwrap_or(c).to_uppercase(),
        _ => String::new(),
    }
}

fn audio_codec_label(acodec: &str) -> String {
    if acodec.starts_with("opus") {
        "Opus".into()
    } else if acodec.starts_with("mp4a") {
        "AAC".into()
    } else if acodec.starts_with("mp3") {
        "MP3".into()
    } else {
        let name = acodec.split('.').next().unwrap_or(acodec);
        let mut chars = name.chars();
        match chars.next() {
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            None => String::new(),
        }
    }
}

fn cmp_f64(a: Option<f64>, b: Option<f64>) -> Ordering {
    a.unwrap_or(0.0).partial_cmp(&b.unwrap_or(0.0)).unwrap_or(Ordering::Equal)
}

/// One choice per distinct height, highest first. At equal height a video-only
/// stream beats a muxed one, then the higher bitrate wins.
pub fn video_choices(info: &VideoInfo) -> Vec<VideoChoice> {
    let mut best: std::collections::BTreeMap<u32, &Format> = std::collections::BTreeMap::new();
    for f in info.formats.iter().filter(|f| has_video(f)) {
        let Some(h) = f.height else { continue };
        let better = match best.get(&h) {
            None => true,
            Some(cur) => {
                let (fa, ca) = (!has_audio(f), !has_audio(cur));
                if fa != ca { fa } else { cmp_f64(f.tbr, cur.tbr) == Ordering::Greater }
            }
        };
        if better {
            best.insert(h, f);
        }
    }
    best.into_iter()
        .rev()
        .map(|(height, f)| VideoChoice {
            format_id: f.format_id.clone(),
            height,
            codec_label: video_codec_label(f.vcodec.as_deref()),
            size_bytes: size_of(f),
        })
        .collect()
}

/// Audio-only streams, highest bitrate first.
pub fn audio_choices(info: &VideoInfo) -> Vec<AudioChoice> {
    let mut v: Vec<&Format> = info
        .formats
        .iter()
        .filter(|f| has_audio(f) && !has_video(f))
        .collect();
    v.sort_by(|a, b| cmp_f64(b.abr.or(b.tbr), a.abr.or(a.tbr)));
    v.into_iter()
        .map(|f| {
            let abr = f.abr.or(f.tbr).map(|x| x.round() as u32).unwrap_or(0);
            let codec = audio_codec_label(f.acodec.as_deref().unwrap_or(""));
            let label = if abr > 0 { format!("{codec} {abr}k") } else { codec };
            AudioChoice {
                format_id: f.format_id.clone(),
                abr,
                codec_label: label,
                size_bytes: size_of(f),
            }
        })
        .collect()
}

/// Estimated download size. `None` as soon as any chosen part has unknown size.
pub fn estimate_total(info: &VideoInfo, v: &VideoSelect, a: &AudioSelect) -> Option<u64> {
    let vids = video_choices(info);
    let auds = audio_choices(info);
    let mut parts: Vec<Option<u64>> = Vec::new();
    match v {
        VideoSelect::None => {}
        VideoSelect::Best => {
            if let Some(c) = vids.first() {
                parts.push(c.size_bytes);
            }
        }
        VideoSelect::Format(id) => parts.push(vids.iter().find(|c| &c.format_id == id)?.size_bytes),
    }
    match a {
        AudioSelect::Best => {
            if let Some(c) = auds.first() {
                parts.push(c.size_bytes);
            }
        }
        AudioSelect::Format(id) => parts.push(auds.iter().find(|c| &c.format_id == id)?.size_bytes),
    }
    if parts.is_empty() {
        return None;
    }
    parts.into_iter().sum::<Option<u64>>()
}

/// Run `yt-dlp -J` for one URL.
pub async fn fetch(ytdlp: &Path, url: &str, cookies: Option<&str>) -> Result<VideoInfo, UserError> {
    let mut cmd = tokio::process::Command::new(ytdlp);
    cmd.args(["-J", "--no-playlist", "--playlist-items", "1"]);
    if let Some(browser) = cookies {
        cmd.args(["--cookies-from-browser", browser]);
    }
    cmd.arg(url).stdin(Stdio::null()).kill_on_drop(true);
    let out = cmd
        .output()
        .await
        .map_err(|e| UserError::new(ErrorKind::ToolMissing, format!("cannot run yt-dlp: {e}")))?;
    if !out.status.success() {
        return Err(crate::errors_classify::classify_stderr(&String::from_utf8_lossy(&out.stderr)));
    }
    parse_probe(&String::from_utf8_lossy(&out.stdout))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorKind;
    use crate::model::{AudioSelect, VideoSelect};

    fn load(name: &str) -> VideoInfo {
        let p = format!("{}/tests/fixtures/{}", env!("CARGO_MANIFEST_DIR"), name);
        parse_probe(&std::fs::read_to_string(p).unwrap()).unwrap()
    }

    #[test]
    fn normalize_url_trims_and_rejects_text() {
        assert_eq!(normalize_url("  https://a.b/c\n").unwrap(), "https://a.b/c");
        assert_eq!(normalize_url("привіт").unwrap_err().kind, ErrorKind::InvalidUrl);
        assert_eq!(normalize_url("https://a.b/c d").unwrap_err().kind, ErrorKind::InvalidUrl);
        assert_eq!(normalize_url("   ").unwrap_err().kind, ErrorKind::InvalidUrl);
    }

    #[test]
    fn video_choices_one_per_height_sorted_desc() {
        let v = video_choices(&load("probe_full.json"));
        let heights: Vec<u32> = v.iter().map(|c| c.height).collect();
        assert_eq!(heights, vec![2160, 1080, 720, 360]);
        let c1080 = v.iter().find(|c| c.height == 1080).unwrap();
        assert_eq!(c1080.format_id, "137");
        assert_eq!(c1080.size_bytes, Some(300_000_000));
        assert_eq!(v.iter().find(|c| c.height == 720).unwrap().size_bytes, Some(150_000_000));
        // video-only preferred over the muxed format 18 at the same height
        assert_eq!(v.iter().find(|c| c.height == 360).unwrap().format_id, "134");
    }

    #[test]
    fn audio_choices_sorted_by_abr() {
        let a = audio_choices(&load("probe_full.json"));
        let ids: Vec<&str> = a.iter().map(|c| c.format_id.as_str()).collect();
        assert_eq!(ids, vec!["251", "140"]);
        assert_eq!(a[0].codec_label, "Opus 160k");
        assert_eq!(a[1].codec_label, "AAC 128k");
    }

    #[test]
    fn estimate_total_best_adds_best_video_and_audio() {
        let i = load("probe_full.json");
        assert_eq!(estimate_total(&i, &VideoSelect::Best, &AudioSelect::Best), Some(1_505_000_000));
        assert_eq!(
            estimate_total(&i, &VideoSelect::Format("137".into()), &AudioSelect::Format("140".into())),
            Some(304_000_000)
        );
        assert_eq!(estimate_total(&i, &VideoSelect::None, &AudioSelect::Best), Some(5_000_000));
    }

    #[test]
    fn combined_only_still_offers_video_choice() {
        let i = load("probe_combined_only.json");
        let v = video_choices(&i);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].height, 360);
        assert!(audio_choices(&i).is_empty());
        assert_eq!(estimate_total(&i, &VideoSelect::Best, &AudioSelect::Best), Some(40_000_000));
    }

    #[test]
    fn audio_only_source_has_no_video_choices() {
        let i = load("probe_audio_only.json");
        assert!(video_choices(&i).is_empty());
        assert_eq!(audio_choices(&i).len(), 2);
    }

    #[test]
    fn unknown_size_is_none_not_zero() {
        let i = load("probe_unknown_size.json");
        assert!(video_choices(&i).iter().all(|c| c.size_bytes.is_none()));
        assert!(audio_choices(&i).iter().all(|c| c.size_bytes.is_none()));
        assert_eq!(estimate_total(&i, &VideoSelect::Best, &AudioSelect::Best), None);
    }

    #[test]
    fn playlist_is_flagged() {
        let json = r#"{"_type":"playlist","id":"PL1","title":"List","webpage_url":"https://x.y/pl",
            "entries":[{"id":"v1","title":"First","webpage_url":"https://x.y/v1","formats":[
            {"format_id":"18","ext":"mp4","vcodec":"avc1","acodec":"mp4a","height":360}]}]}"#;
        let i = parse_probe(json).unwrap();
        assert!(i.is_playlist);
        assert_eq!(i.formats.len(), 1);
    }

    #[test]
    fn parse_rejects_garbage() {
        assert_eq!(parse_probe("not json").unwrap_err().kind, ErrorKind::Other);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn fetch_parses_stdout_and_reports_failure() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let fixture = format!("{}/tests/fixtures/probe_combined_only.json", env!("CARGO_MANIFEST_DIR"));
        let ok = dir.path().join("ok.sh");
        std::fs::write(&ok, format!("#!/bin/sh\ncat '{fixture}'\n")).unwrap();
        let bad = dir.path().join("bad.sh");
        std::fs::write(&bad, "#!/bin/sh\necho 'boom' >&2\nexit 1\n").unwrap();
        for p in [&ok, &bad] {
            std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let info = fetch(&ok, "https://site.example/v/c1", None).await.unwrap();
        assert_eq!(info.id, "c1");
        let err = fetch(&bad, "https://site.example/v/c1", None).await.unwrap_err();
        assert!(err.detail.contains("boom"), "{err:?}");
        let login = dir.path().join("login.sh");
        std::fs::write(&login, "#!/bin/sh\necho 'ERROR: Sign in to confirm' >&2\nexit 1\n").unwrap();
        std::fs::set_permissions(&login, std::fs::Permissions::from_mode(0o755)).unwrap();
        let err = fetch(&login, "https://site.example/v/c1", None).await.unwrap_err();
        assert_eq!(err.kind, ErrorKind::LoginRequired);
    }
}
