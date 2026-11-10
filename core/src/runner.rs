use std::path::PathBuf;
use std::process::Stdio;

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

use crate::args::build_download_args;
use crate::error::{ErrorKind, UserError};
use crate::errors_classify::classify_stderr;
use crate::model::{Container, DownloadRequest, Progress};
use crate::proc;
use crate::progress::{parse_line, LineEvent};

#[derive(Debug, Clone, PartialEq)]
pub enum RunEvent {
    Progress(Progress),
    Processing,
    File(PathBuf),
}

#[derive(Debug)]
pub enum RunError {
    Cancelled,
    Failed(UserError),
}

/// Runs one download. Implemented by [`YtDlpRunner`]; the queue only knows this trait.
#[async_trait]
pub trait JobRunner: Send + Sync {
    async fn run(
        &self,
        req: &DownloadRequest,
        on_event: &(dyn Fn(RunEvent) + Send + Sync),
        cancel: CancellationToken,
    ) -> Result<PathBuf, RunError>;
}

pub struct YtDlpRunner {
    pub ytdlp: PathBuf,
    pub ffmpeg: Option<PathBuf>,
    pub out_dir: PathBuf,
    pub container: Container,
    pub cookies: Option<String>,
}

fn failed(kind: ErrorKind, detail: impl Into<String>) -> RunError {
    RunError::Failed(UserError::new(kind, detail))
}

#[async_trait]
impl JobRunner for YtDlpRunner {
    async fn run(
        &self,
        req: &DownloadRequest,
        on_event: &(dyn Fn(RunEvent) + Send + Sync),
        cancel: CancellationToken,
    ) -> Result<PathBuf, RunError> {
        let args = build_download_args(
            req,
            &self.out_dir,
            self.container,
            self.ffmpeg.as_deref(),
            self.cookies.as_deref(),
        );
        let mut cmd = Command::new(&self.ytdlp);
        cmd.args(&args)
            .env("PYTHONIOENCODING", "utf-8")
            .env("PYTHONUTF8", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        proc::configure(&mut cmd);
        let mut child = cmd
            .spawn()
            .map_err(|e| failed(ErrorKind::ToolMissing, format!("cannot run yt-dlp: {e}")))?;

        let stdout = child.stdout.take().expect("stdout is piped");
        let mut stderr = child.stderr.take().expect("stderr is piped");
        let err_task = tokio::spawn(async move {
            let mut buf = Vec::new();
            let _ = stderr.read_to_end(&mut buf).await;
            String::from_utf8_lossy(&buf).into_owned()
        });

        let mut reader = BufReader::new(stdout);
        let mut line = Vec::new();
        let mut file: Option<PathBuf> = None;
        loop {
            line.clear();
            tokio::select! {
                _ = cancel.cancelled() => {
                    proc::terminate(&mut child, proc::GRACE).await;
                    err_task.abort();
                    return Err(RunError::Cancelled);
                }
                n = reader.read_until(b'\n', &mut line) => match n {
                    Ok(0) | Err(_) => break,
                    Ok(_) => match parse_line(&String::from_utf8_lossy(&line)) {
                        LineEvent::Progress(p) => on_event(RunEvent::Progress(p)),
                        LineEvent::PostProcess(_) => on_event(RunEvent::Processing),
                        LineEvent::File(f) => {
                            file = Some(f.clone());
                            on_event(RunEvent::File(f));
                        }
                        LineEvent::Other => {}
                    },
                }
            }
        }

        let status = tokio::select! {
            _ = cancel.cancelled() => {
                proc::terminate(&mut child, proc::GRACE).await;
                err_task.abort();
                return Err(RunError::Cancelled);
            }
            s = child.wait() => s,
        };
        let stderr_text = err_task.await.unwrap_or_default();
        match status {
            Ok(s) if s.success() => file
                .ok_or_else(|| failed(ErrorKind::Other, "yt-dlp finished without reporting an output file")),
            Ok(s) => {
                let mut e = classify_stderr(&stderr_text);
                if e.detail.is_empty() {
                    e.detail = format!("yt-dlp exited with {s}");
                }
                Err(RunError::Failed(e))
            }
            Err(e) => Err(failed(ErrorKind::Other, e.to_string())),
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::error::ErrorKind;
    use crate::model::{AudioOnlyFormat, AudioSelect, Container, DownloadRequest, VideoSelect};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    fn runner(out: &std::path::Path) -> YtDlpRunner {
        YtDlpRunner {
            ytdlp: format!("{}/tests/fixtures/fake_ytdlp.sh", env!("CARGO_MANIFEST_DIR")).into(),
            ffmpeg: None,
            out_dir: out.to_path_buf(),
            container: Container::Mp4,
            cookies: None,
        }
    }

    fn req(mode: &str) -> DownloadRequest {
        DownloadRequest {
            url: format!("https://t.example/{mode}"),
            title: "t".into(),
            thumbnail: None,
            video: VideoSelect::Best,
            audio: AudioSelect::Best,
            audio_only_format: AudioOnlyFormat::Mp3,
        }
    }

    async fn run(mode: &str) -> (Result<std::path::PathBuf, RunError>, Vec<String>) {
        let dir = tempfile::tempdir().unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = events.clone();
        let on_event = move |e: RunEvent| {
            let tag = match e {
                RunEvent::Progress(p) => format!("progress:{}", p.downloaded),
                RunEvent::Processing => "processing".to_string(),
                RunEvent::File(f) => format!("file:{}", f.display()),
            };
            sink.lock().unwrap().push(tag);
        };
        let res = runner(dir.path()).run(&req(mode), &on_event, CancellationToken::new()).await;
        let seen = events.lock().unwrap().clone();
        (res, seen)
    }

    #[tokio::test]
    async fn run_streams_progress_then_file() {
        let (res, events) = run("ok").await;
        assert_eq!(res.unwrap(), std::path::PathBuf::from("/tmp/x y.mp4"));
        assert_eq!(events, vec!["progress:100", "progress:1000", "processing", "file:/tmp/x y.mp4"]);
    }

    #[tokio::test]
    async fn run_failure_is_classified() {
        let (res, _) = run("fail_login").await;
        match res {
            Err(RunError::Failed(e)) => assert_eq!(e.kind, ErrorKind::LoginRequired),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[tokio::test]
    async fn exit_zero_without_file_line_is_an_error() {
        let (res, _) = run("nofile").await;
        assert!(matches!(res, Err(RunError::Failed(_))), "{res:?}");
    }

    #[tokio::test]
    async fn already_downloaded_file_counts_as_done() {
        let (res, _) = run("exists").await;
        assert_eq!(res.unwrap(), std::path::PathBuf::from("/tmp/My Clip.mp4"));
    }

    #[tokio::test]
    async fn cancel_kills_child() {
        let dir = tempfile::tempdir().unwrap();
        let token = CancellationToken::new();
        let t = token.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(200)).await;
            t.cancel();
        });
        let started = Instant::now();
        let res = runner(dir.path()).run(&req("slow"), &|_| {}, token).await;
        assert!(matches!(res, Err(RunError::Cancelled)), "{res:?}");
        assert!(started.elapsed() < Duration::from_secs(2), "child was not killed promptly");
    }

    fn alive(pid: &str) -> bool {
        std::process::Command::new("kill").args(["-0", pid]).stderr(std::process::Stdio::null()).status().map(|s| s.success()).unwrap_or(false)
    }

    #[tokio::test]
    async fn cancel_kills_the_whole_process_tree() {
        let dir = tempfile::tempdir().unwrap();
        let token = CancellationToken::new();
        let t = token.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(500)).await;
            t.cancel();
        });
        let res = runner(dir.path()).run(&req("tree"), &|_| {}, token).await;
        assert!(matches!(res, Err(RunError::Cancelled)), "{res:?}");
        let pid = std::fs::read_to_string(dir.path().join("child.pid")).unwrap().trim().to_string();
        let mut gone = false;
        for _ in 0..40 {
            if !alive(&pid) { gone = true; break; }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        if !gone { let _ = std::process::Command::new("kill").args(["-9", &pid]).status(); }
        assert!(gone, "child process {pid} (ffmpeg stand-in) survived the cancel");
    }

    #[tokio::test]
    async fn missing_binary_reports_tool_missing() {
        let dir = tempfile::tempdir().unwrap();
        let mut r = runner(dir.path());
        r.ytdlp = dir.path().join("does-not-exist");
        match r.run(&req("ok"), &|_| {}, CancellationToken::new()).await {
            Err(RunError::Failed(e)) => assert_eq!(e.kind, ErrorKind::ToolMissing),
            other => panic!("unexpected {other:?}"),
        }
    }
}
