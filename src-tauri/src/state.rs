use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use streamflow_core::error::{ErrorKind, UserError};
use streamflow_core::model::DownloadRequest;
use streamflow_core::queue::Queue;
use streamflow_core::runner::{JobRunner, RunError, RunEvent, YtDlpRunner};
use streamflow_core::settings::Settings;
use streamflow_core::tools::{self, Platform, ToolId, ToolsStatus};
use tokio_util::sync::CancellationToken;

pub const APP_NAME: &str = "StreamFlow";

/// Settings plus tool locations. Everything the commands need to build a runner.
pub struct AppCtx {
    pub tools_dir: PathBuf,
    pub settings_path: PathBuf,
    settings: Mutex<Settings>,
    path_var: Option<OsString>,
}

impl AppCtx {
    pub fn new(tools_dir: PathBuf, settings_path: PathBuf) -> AppCtx {
        let settings = Settings::load(&settings_path);
        AppCtx { tools_dir, settings_path, settings: Mutex::new(settings), path_var: Some(tools::search_path()) }
    }

    /// Replace the PATH used to look for tools (tests use this for isolation).
    pub fn with_path_var(mut self, path_var: Option<OsString>) -> AppCtx {
        self.path_var = path_var;
        self
    }

    pub fn settings(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }

    /// Store and persist new settings; returns the values actually applied.
    pub fn update_settings(&self, mut new: Settings) -> std::io::Result<Settings> {
        new.max_parallel = new.max_parallel.clamp(1, 6);
        new.save(&self.settings_path)?;
        *self.settings.lock().unwrap() = new.clone();
        Ok(new)
    }

    pub fn locate(&self, tool: ToolId) -> Option<PathBuf> {
        tools::locate_in(tool, &self.tools_dir, self.path_var.as_deref())
    }

    pub fn tools_status(&self) -> ToolsStatus {
        ToolsStatus { ytdlp: self.locate(ToolId::YtDlp), ffmpeg: self.locate(ToolId::Ffmpeg) }
    }

    pub fn ytdlp_path(&self) -> Result<PathBuf, UserError> {
        self.locate(ToolId::YtDlp).ok_or_else(|| {
            UserError::new(ErrorKind::ToolMissing, tools::manual_hint(ToolId::YtDlp, Platform::current()))
        })
    }

    pub fn build_runner(&self) -> Result<YtDlpRunner, UserError> {
        let s = self.settings();
        Ok(YtDlpRunner {
            ytdlp: self.ytdlp_path()?,
            ffmpeg: self.locate(ToolId::Ffmpeg),
            out_dir: s.effective_output_dir(),
            container: s.container,
            cookies: s.cookies_arg().map(str::to_owned),
        })
    }
}

/// Builds a fresh [`YtDlpRunner`] for every job so settings and tool installs apply immediately.
pub struct ConfiguredRunner(pub Arc<AppCtx>);

#[async_trait]
impl JobRunner for ConfiguredRunner {
    async fn run(
        &self,
        req: &DownloadRequest,
        on_event: &(dyn Fn(RunEvent) + Send + Sync),
        cancel: CancellationToken,
    ) -> Result<PathBuf, RunError> {
        match self.0.build_runner() {
            Ok(runner) => runner.run(req, on_event, cancel).await,
            Err(e) => Err(RunError::Failed(e)),
        }
    }
}

pub struct AppState {
    pub ctx: Arc<AppCtx>,
    pub queue: Queue,
}

#[cfg(test)]
mod tests {
    use super::*;
    use streamflow_core::error::ErrorKind;
    use streamflow_core::model::{AudioOnlyFormat, AudioSelect, Container, DownloadRequest, VideoSelect};
    use streamflow_core::settings::{CookieSource, Settings};
    use streamflow_core::tools::{Platform, ToolId};

    fn ctx(tools: &std::path::Path, cfg: &std::path::Path, empty_path: &std::path::Path) -> AppCtx {
        AppCtx::new(tools.to_path_buf(), cfg.join("settings.json")).with_path_var(Some(empty_path.as_os_str().to_owned()))
    }

    fn fake_tool(dir: &std::path::Path, tool: ToolId) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join(tool.file_name(Platform::current().os)), b"x").unwrap();
    }

    fn req() -> DownloadRequest {
        DownloadRequest {
            url: "https://site.example/v".into(),
            title: "t".into(),
            thumbnail: None,
            video: VideoSelect::Best,
            audio: AudioSelect::Best,
            audio_only_format: AudioOnlyFormat::Mp3,
        }
    }

    #[test]
    fn build_runner_reports_tool_missing_without_ytdlp() {
        let (t, c, e) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let err = ctx(t.path(), c.path(), e.path()).build_runner().err().expect("must fail");
        assert_eq!(err.kind, ErrorKind::ToolMissing);
        assert!(err.detail.contains("yt-dlp"), "{err:?}");
    }

    #[test]
    fn build_runner_uses_settings_and_found_tools() {
        let (t, c, e) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        fake_tool(t.path(), ToolId::YtDlp);
        fake_tool(t.path(), ToolId::Ffmpeg);
        let app = ctx(t.path(), c.path(), e.path());
        let out = c.path().join("videos");
        app.update_settings(Settings {
            output_dir: Some(out.clone()),
            container: Container::Mkv,
            cookies_from: CookieSource::Safari,
            ..Settings::default()
        })
        .unwrap();
        let r = app.build_runner().unwrap();
        assert_eq!(r.out_dir, out);
        assert_eq!(r.container, Container::Mkv);
        assert_eq!(r.cookies.as_deref(), Some("safari"));
        assert!(r.ffmpeg.is_some());
    }

    #[test]
    fn ffmpeg_is_optional() {
        let (t, c, e) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        fake_tool(t.path(), ToolId::YtDlp);
        let r = ctx(t.path(), c.path(), e.path()).build_runner().unwrap();
        assert!(r.ffmpeg.is_none());
        assert_eq!(r.out_dir, streamflow_core::paths::downloads_dir());
    }

    #[test]
    fn update_settings_persists_and_clamps() {
        let (t, c, e) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let app = ctx(t.path(), c.path(), e.path());
        let saved = app.update_settings(Settings { max_parallel: 99, ..Settings::default() }).unwrap();
        assert_eq!(saved.max_parallel, 6);
        assert_eq!(app.settings().max_parallel, 6);
        assert_eq!(Settings::load(&c.path().join("settings.json")).max_parallel, 6);
    }

    #[tokio::test]
    async fn configured_runner_fails_with_tool_missing() {
        use streamflow_core::runner::{JobRunner, RunError};
        let (t, c, e) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let runner = ConfiguredRunner(std::sync::Arc::new(ctx(t.path(), c.path(), e.path())));
        match runner.run(&req(), &|_| {}, tokio_util::sync::CancellationToken::new()).await {
            Err(RunError::Failed(err)) => assert_eq!(err.kind, ErrorKind::ToolMissing),
            other => panic!("unexpected {other:?}"),
        }
    }
}
