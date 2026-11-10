//! Real-network checks against the actual yt-dlp and ffmpeg binaries.
//! Run with: cargo test -p streamflow-core --test integration_real -- --ignored --nocapture
//! Set SF_TEST_URL to use another source (default: the first YouTube video, 19 s).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use streamflow_core::model::{AudioOnlyFormat, AudioSelect, Container, DownloadRequest, VideoSelect};
use streamflow_core::probe::{audio_choices, fetch, video_choices};
use streamflow_core::runner::{JobRunner, RunEvent, YtDlpRunner};
use streamflow_core::tools::{install::install, Platform, ToolId};
use tokio_util::sync::CancellationToken;

fn test_url() -> String {
    std::env::var("SF_TEST_URL").unwrap_or_else(|_| "https://www.youtube.com/watch?v=jNQXAC9IVRw".into())
}

async fn tools(dir: &Path) -> (PathBuf, PathBuf) {
    let p = Platform::current();
    let y = install(ToolId::YtDlp, p, dir, &|_, _| {}).await.expect("install yt-dlp");
    let f = install(ToolId::Ffmpeg, p, dir, &|_, _| {}).await.expect("install ffmpeg");
    (y, f)
}

fn request(video: VideoSelect, audio_only_format: AudioOnlyFormat) -> DownloadRequest {
    DownloadRequest { url: test_url(), title: "t".into(), thumbnail: None, video, audio: AudioSelect::Best, audio_only_format }
}

fn streams(ffmpeg: &Path, file: &Path) -> String {
    let out = std::process::Command::new(ffmpeg).arg("-i").arg(file).output().expect("run ffmpeg");
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[tokio::test]
#[ignore = "needs network"]
async fn real_download_has_video_and_audio() {
    let (tools_dir, out_dir) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let (ytdlp, ffmpeg) = tools(tools_dir.path()).await;

    let info = fetch(&ytdlp, &test_url(), None).await.expect("probe");
    eprintln!("title={:?} video={:?} audio={:?}", info.title, video_choices(&info).len(), audio_choices(&info).len());
    assert!(!video_choices(&info).is_empty());

    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = events.clone();
    let runner = YtDlpRunner {
        ytdlp,
        ffmpeg: Some(ffmpeg.clone()),
        out_dir: out_dir.path().to_path_buf(),
        container: Container::Mp4,
        cookies: None,
    };
    let file = runner
        .run(&request(VideoSelect::Best, AudioOnlyFormat::Mp3), &move |e: RunEvent| sink.lock().unwrap().push(e), CancellationToken::new())
        .await
        .expect("download");

    assert!(file.exists(), "{file:?} missing");
    assert_eq!(file.extension().and_then(|e| e.to_str()), Some("mp4"));
    let info = streams(&ffmpeg, &file);
    assert!(info.contains("Video:") && info.contains("Audio:"), "one file must hold both streams:\n{info}");
    let progress = events.lock().unwrap().iter().filter(|e| matches!(e, RunEvent::Progress(_))).count();
    assert!(progress > 0, "no progress events were parsed from real yt-dlp output");
}

#[tokio::test]
#[ignore = "needs network"]
async fn real_audio_only_mp3() {
    let (tools_dir, out_dir) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let (ytdlp, ffmpeg) = tools(tools_dir.path()).await;
    let runner = YtDlpRunner {
        ytdlp,
        ffmpeg: Some(ffmpeg.clone()),
        out_dir: out_dir.path().to_path_buf(),
        container: Container::Mp4,
        cookies: None,
    };
    let file = runner
        .run(&request(VideoSelect::None, AudioOnlyFormat::Mp3), &|_| {}, CancellationToken::new())
        .await
        .expect("download");
    assert_eq!(file.extension().and_then(|e| e.to_str()), Some("mp3"), "{file:?}");
    let info = streams(&ffmpeg, &file);
    assert!(info.contains("Audio:") && !info.contains("Video: h264"), "{info}");
}
