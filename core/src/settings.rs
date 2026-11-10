use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::model::Container;
use crate::paths::downloads_dir;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    Auto,
    Uk,
    En,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Auto,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CookieSource {
    #[default]
    None,
    Safari,
    Chrome,
    Firefox,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub output_dir: Option<PathBuf>,
    pub language: Language,
    pub theme: Theme,
    pub max_parallel: u8,
    pub cookies_from: CookieSource,
    pub container: Container,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            output_dir: None,
            language: Language::Auto,
            theme: Theme::Auto,
            max_parallel: 2,
            cookies_from: CookieSource::None,
            container: Container::Mp4,
        }
    }
}

impl Settings {
    /// Read settings; a missing or unreadable file yields the defaults.
    pub fn load(path: &Path) -> Settings {
        let mut s: Settings = std::fs::read_to_string(path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        s.max_parallel = s.max_parallel.clamp(1, 6);
        s
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self)?)
    }

    pub fn effective_output_dir(&self) -> PathBuf {
        self.output_dir.clone().unwrap_or_else(downloads_dir)
    }

    pub fn cookies_arg(&self) -> Option<&'static str> {
        match self.cookies_from {
            CookieSource::None => None,
            CookieSource::Safari => Some("safari"),
            CookieSource::Chrome => Some("chrome"),
            CookieSource::Firefox => Some("firefox"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Container;
    use crate::paths::downloads_dir;

    #[test]
    fn defaults() {
        let s = Settings::default();
        assert_eq!(s.max_parallel, 2);
        assert_eq!(s.container, Container::Mp4);
        assert_eq!(s.effective_output_dir(), downloads_dir());
        assert_eq!(s.cookies_arg(), None);
    }

    #[test]
    fn roundtrip_and_custom_output_dir() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub/settings.json");
        let mut s = Settings::default();
        s.output_dir = Some(dir.path().join("videos"));
        s.container = Container::Mkv;
        s.cookies_from = CookieSource::Chrome;
        s.save(&path).unwrap();
        let back = Settings::load(&path);
        assert_eq!(back, s);
        assert_eq!(back.effective_output_dir(), dir.path().join("videos"));
    }

    #[test]
    fn missing_and_corrupt_files_give_defaults() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(Settings::load(&dir.path().join("nope.json")), Settings::default());
        let bad = dir.path().join("bad.json");
        std::fs::write(&bad, "{not json").unwrap();
        assert_eq!(Settings::load(&bad), Settings::default());
    }

    #[test]
    fn max_parallel_is_clamped_on_load() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("s.json");
        std::fs::write(&p, r#"{"maxParallel":0}"#).unwrap();
        assert_eq!(Settings::load(&p).max_parallel, 1);
        std::fs::write(&p, r#"{"maxParallel":20}"#).unwrap();
        assert_eq!(Settings::load(&p).max_parallel, 6);
    }

    #[test]
    fn cookies_arg_values() {
        let mut s = Settings::default();
        s.cookies_from = CookieSource::Safari;
        assert_eq!(s.cookies_arg(), Some("safari"));
        s.cookies_from = CookieSource::Firefox;
        assert_eq!(s.cookies_arg(), Some("firefox"));
    }
}
