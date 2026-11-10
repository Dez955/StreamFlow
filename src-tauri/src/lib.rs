mod commands;
mod state;

use std::sync::Arc;

use streamflow_core::model::JobState;
use streamflow_core::paths::{settings_path, tools_dir};
use streamflow_core::queue::{JobSnapshot, Queue};
use tauri::{Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

use state::{AppCtx, AppState, ConfiguredRunner, APP_NAME};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let ctx = Arc::new(AppCtx::new(tools_dir(APP_NAME), settings_path(APP_NAME)));
            let handle = app.handle().clone();
            let max = ctx.settings().max_parallel as usize;
            let runner = Arc::new(ConfiguredRunner(ctx.clone()));
            let queue = tauri::async_runtime::block_on(async move {
                Queue::new(
                    runner,
                    max,
                    Arc::new(move |snap: JobSnapshot| {
                        let _ = handle.emit("job-changed", &snap);
                        if snap.state == JobState::Done {
                            let _ = handle.notification().builder().title("StreamFlow").body(&snap.request.title).show();
                        }
                    }),
                )
            });
            app.manage(AppState { ctx, queue });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::analyze,
            commands::start,
            commands::pause,
            commands::resume,
            commands::stop,
            commands::remove,
            commands::list_jobs,
            commands::open_job,
            commands::get_settings,
            commands::set_settings,
            commands::tools_status,
            commands::tools_hint,
            commands::tools_install,
            commands::tools_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running StreamFlow");
}
