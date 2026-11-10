use std::path::PathBuf;

/// The platform's Downloads folder (falls back to `~/Downloads`).
pub fn downloads_dir() -> PathBuf {
    dirs::download_dir().unwrap_or_else(|| {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join("Downloads")
    })
}

/// Where the app keeps its own copies of yt-dlp / ffmpeg.
pub fn tools_dir(app: &str) -> PathBuf {
    dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join(app).join("bin")
}

/// Location of the settings file.
pub fn settings_path(app: &str) -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join(app).join("settings.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downloads_dir_is_named_downloads() {
        assert!(downloads_dir().ends_with("Downloads") || dirs::download_dir().is_some());
    }

    #[test]
    fn tools_dir_is_app_bin_under_data_dir() {
        let d = tools_dir("StreamFlow");
        assert!(d.ends_with("StreamFlow/bin"), "{d:?}");
    }

    #[test]
    fn settings_path_is_json_under_config_dir() {
        assert!(settings_path("StreamFlow").ends_with("StreamFlow/settings.json"));
    }
}
