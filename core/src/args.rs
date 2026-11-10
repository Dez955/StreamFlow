use std::path::Path;

use crate::model::{AudioOnlyFormat, AudioSelect, Container, DownloadRequest, VideoSelect};

pub const PROGRESS_PREFIX: &str = "sf-progress:";
pub const POST_PREFIX: &str = "sf-post:";
pub const FILE_PREFIX: &str = "sf-file:";

/// Arguments for one yt-dlp download. The URL is always the last argument.
pub fn build_download_args(
    req: &DownloadRequest,
    out_dir: &Path,
    container: Container,
    ffmpeg: Option<&Path>,
    cookies: Option<&str>,
) -> Vec<String> {
    let mut a: Vec<String> = vec![
        "--no-playlist".into(),
        "--newline".into(),
        "--progress".into(),
        "--no-warnings".into(),
        "--progress-template".into(),
        format!(
            "download:{PROGRESS_PREFIX}%(progress.downloaded_bytes)s|%(progress.total_bytes)s|%(progress.total_bytes_estimate)s|%(progress.speed)s|%(progress.eta)s"
        ),
        "--progress-template".into(),
        format!("postprocess:{POST_PREFIX}%(progress.postprocessor)s"),
        "--print".into(),
        format!("after_move:{FILE_PREFIX}%(filepath)s"),
        "-o".into(),
        out_dir.join("%(title)s.%(ext)s").to_string_lossy().into_owned(),
    ];

    match &req.video {
        VideoSelect::None => {
            let fmt = match req.audio_only_format {
                AudioOnlyFormat::Mp3 => "mp3",
                AudioOnlyFormat::M4a => "m4a",
                AudioOnlyFormat::Opus => "opus",
            };
            let selector = match &req.audio {
                AudioSelect::Best => "ba/b".to_string(),
                AudioSelect::Format(id) => id.clone(),
            };
            a.extend(["-x".into(), "--audio-format".into(), fmt.into()]);
            a.extend(["--audio-quality".into(), "0".into(), "-f".into(), selector]);
        }
        video => {
            let (v, v_best) = match video {
                VideoSelect::Format(id) => (id.as_str(), false),
                _ => ("bv*", true),
            };
            let (au, a_best) = match &req.audio {
                AudioSelect::Format(id) => (id.as_str(), false),
                AudioSelect::Best => ("ba", true),
            };
            let fallback = if v_best || a_best { "/b" } else { "" };
            a.extend(["-f".into(), format!("{v}+{au}{fallback}")]);
            let merge = match container {
                Container::Mp4 => "mp4",
                Container::Mkv => "mkv",
            };
            a.extend(["--merge-output-format".into(), merge.into()]);
        }
    }

    if let Some(path) = ffmpeg {
        a.extend(["--ffmpeg-location".into(), path.to_string_lossy().into_owned()]);
    }
    if let Some(browser) = cookies {
        a.extend(["--cookies-from-browser".into(), browser.into()]);
    }
    a.push(req.url.clone());
    a
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AudioOnlyFormat, AudioSelect, Container, DownloadRequest, VideoSelect};
    use std::path::Path;

    fn req(video: VideoSelect, audio: AudioSelect) -> DownloadRequest {
        DownloadRequest {
            url: "https://site.example/v".into(),
            title: "t".into(),
            thumbnail: None,
            video,
            audio,
            audio_only_format: AudioOnlyFormat::Mp3,
        }
    }

    fn build(r: &DownloadRequest, c: Container) -> Vec<String> {
        build_download_args(r, Path::new("/out"), c, None, None)
    }

    fn has_pair(args: &[String], a: &str, b: &str) -> bool {
        args.windows(2).any(|w| w[0] == a && w[1] == b)
    }

    #[test]
    fn best_default() {
        let a = build(&req(VideoSelect::Best, AudioSelect::Best), Container::Mp4);
        assert!(has_pair(&a, "-f", "bv*+ba/b"), "{a:?}");
        assert!(has_pair(&a, "--merge-output-format", "mp4"));
        assert!(has_pair(&a, "-o", "/out/%(title)s.%(ext)s"));
        assert_eq!(a.last().unwrap(), "https://site.example/v");
    }

    #[test]
    fn mkv_container() {
        let a = build(&req(VideoSelect::Best, AudioSelect::Best), Container::Mkv);
        assert!(has_pair(&a, "--merge-output-format", "mkv"));
    }

    #[test]
    fn explicit_formats() {
        let a = build(&req(VideoSelect::Format("313".into()), AudioSelect::Format("251".into())), Container::Mp4);
        assert!(has_pair(&a, "-f", "313+251"), "{a:?}");
        let b = build(&req(VideoSelect::Format("313".into()), AudioSelect::Best), Container::Mp4);
        assert!(has_pair(&b, "-f", "313+ba/b"), "{b:?}");
        let c = build(&req(VideoSelect::Best, AudioSelect::Format("251".into())), Container::Mp4);
        assert!(has_pair(&c, "-f", "bv*+251/b"), "{c:?}");
    }

    #[test]
    fn audio_only_mp3() {
        let a = build(&req(VideoSelect::None, AudioSelect::Best), Container::Mp4);
        assert!(a.contains(&"-x".to_string()));
        assert!(has_pair(&a, "--audio-format", "mp3"));
        assert!(has_pair(&a, "--audio-quality", "0"));
        assert!(has_pair(&a, "-f", "ba/b"));
        assert!(!a.contains(&"--merge-output-format".to_string()));
        let b = build(&req(VideoSelect::None, AudioSelect::Format("251".into())), Container::Mp4);
        assert!(has_pair(&b, "-f", "251"));
    }

    #[test]
    fn audio_only_other_formats() {
        let mut r = req(VideoSelect::None, AudioSelect::Best);
        r.audio_only_format = AudioOnlyFormat::M4a;
        assert!(has_pair(&build(&r, Container::Mp4), "--audio-format", "m4a"));
        r.audio_only_format = AudioOnlyFormat::Opus;
        assert!(has_pair(&build(&r, Container::Mp4), "--audio-format", "opus"));
    }

    #[test]
    fn ffmpeg_and_cookies() {
        let a = build_download_args(
            &req(VideoSelect::Best, AudioSelect::Best),
            Path::new("/out"),
            Container::Mp4,
            Some(Path::new("/x/ffmpeg")),
            Some("safari"),
        );
        assert!(has_pair(&a, "--ffmpeg-location", "/x/ffmpeg"));
        assert!(has_pair(&a, "--cookies-from-browser", "safari"));
    }

    #[test]
    fn always_no_playlist_and_machine_readable_output() {
        let a = build(&req(VideoSelect::Best, AudioSelect::Best), Container::Mp4);
        for flag in ["--no-playlist", "--newline", "--progress"] {
            assert!(a.contains(&flag.to_string()), "{flag} missing in {a:?}");
        }
        assert!(has_pair(&a, "--print", "after_move:sf-file:%(filepath)s"));
        assert!(a.iter().any(|s| s.starts_with("download:sf-progress:")));
        assert!(a.iter().any(|s| s.starts_with("postprocess:sf-post:")));
    }
}
