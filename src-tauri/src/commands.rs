use serde::Serialize;
use streamflow_core::error::{ErrorKind, UserError};
use streamflow_core::model::{AudioChoice, AudioSelect, DownloadRequest, VideoChoice, VideoInfo, VideoSelect};
use streamflow_core::probe::{audio_choices, estimate_total, fetch, normalize_url, video_choices};
use streamflow_core::queue::JobSnapshot;
use streamflow_core::settings::Settings;
use streamflow_core::tools::{install, Platform, ToolId, ToolsStatus};
use tauri::{AppHandle, Emitter, State};

use crate::state::{AppCtx, AppState};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoProbe {
    pub info: VideoInfo,
    pub video: Vec<VideoChoice>,
    pub audio: Vec<AudioChoice>,
    pub estimate_best: Option<u64>,
}

pub fn probe_from_info(info: VideoInfo) -> VideoProbe {
    VideoProbe {
        video: video_choices(&info),
        audio: audio_choices(&info),
        estimate_best: estimate_total(&info, &VideoSelect::Best, &AudioSelect::Best),
        info,
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ToolsProgress {
    tool: ToolId,
    downloaded: u64,
    total: Option<u64>,
}

fn io_err(e: impl std::fmt::Display) -> UserError {
    UserError::new(ErrorKind::Other, e.to_string())
}

#[tauri::command]
pub async fn analyze(url: String, state: State<'_, AppState>) -> Result<VideoProbe, UserError> {
    let url = normalize_url(&url)?;
    let ytdlp = state.ctx.ytdlp_path()?;
    let cookies = state.ctx.settings().cookies_arg();
    Ok(probe_from_info(fetch(&ytdlp, &url, cookies).await?))
}

#[tauri::command]
pub fn start(req: DownloadRequest, state: State<'_, AppState>) -> String {
    state.queue.add(req)
}

#[tauri::command]
pub fn pause(id: String, state: State<'_, AppState>) {
    state.queue.pause(&id)
}

#[tauri::command]
pub fn resume(id: String, state: State<'_, AppState>) {
    state.queue.resume(&id)
}

#[tauri::command]
pub fn stop(id: String, state: State<'_, AppState>) {
    state.queue.stop(&id)
}

#[tauri::command]
pub fn remove(id: String, state: State<'_, AppState>) {
    state.queue.remove(&id)
}

#[tauri::command]
pub fn list_jobs(state: State<'_, AppState>) -> Vec<JobSnapshot> {
    state.queue.list()
}

/// Show the finished file in Finder / Explorer / the file manager.
#[tauri::command]
pub fn open_job(id: String, state: State<'_, AppState>) -> Result<(), UserError> {
    let job = state.queue.list().into_iter().find(|j| j.id == id);
    let path = job.and_then(|j| j.file_path).ok_or_else(|| io_err("this download has no file yet"))?;
    tauri_plugin_opener::reveal_item_in_dir(path).map_err(io_err)
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.ctx.settings()
}

#[tauri::command]
pub fn set_settings(settings: Settings, state: State<'_, AppState>) -> Result<Settings, UserError> {
    let saved = state.ctx.update_settings(settings).map_err(io_err)?;
    state.queue.set_max_parallel(saved.max_parallel as usize);
    Ok(saved)
}

#[tauri::command]
pub fn tools_status(state: State<'_, AppState>) -> ToolsStatus {
    state.ctx.tools_status()
}

async fn install_tool(app: &AppHandle, ctx: &AppCtx, tool: ToolId) -> Result<ToolsStatus, UserError> {
    let app = app.clone();
    install::install(tool, Platform::current(), &ctx.tools_dir, &move |downloaded, total| {
        let _ = app.emit("tools-progress", ToolsProgress { tool, downloaded, total });
    })
    .await?;
    Ok(ctx.tools_status())
}

#[tauri::command]
pub async fn tools_install(tool: ToolId, app: AppHandle, state: State<'_, AppState>) -> Result<ToolsStatus, UserError> {
    install_tool(&app, &state.ctx, tool).await
}

/// Re-download yt-dlp (sites change often, so updating fixes most extractor errors).
#[tauri::command]
pub async fn tools_update(app: AppHandle, state: State<'_, AppState>) -> Result<ToolsStatus, UserError> {
    install_tool(&app, &state.ctx, ToolId::YtDlp).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use streamflow_core::probe::parse_probe;

    #[test]
    fn probe_payload_has_choices_and_best_estimate() {
        let path = format!("{}/../core/tests/fixtures/probe_full.json", env!("CARGO_MANIFEST_DIR"));
        let info = parse_probe(&std::fs::read_to_string(path).unwrap()).unwrap();
        let p = probe_from_info(info);
        assert_eq!(p.video.iter().map(|c| c.height).collect::<Vec<_>>(), vec![2160, 1080, 720, 360]);
        assert_eq!(p.audio.len(), 2);
        assert_eq!(p.estimate_best, Some(1_505_000_000));
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"estimateBest\""), "{json}");
    }
}
