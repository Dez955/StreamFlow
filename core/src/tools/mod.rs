pub mod install;
pub mod manifest;

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolId {
    YtDlp,
    Ffmpeg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    MacOs,
    Windows,
    Linux,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    X86,
    X64,
    Arm64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Platform {
    pub os: Os,
    pub arch: Arch,
}

impl Platform {
    pub fn current() -> Platform {
        let os = if cfg!(target_os = "macos") {
            Os::MacOs
        } else if cfg!(windows) {
            Os::Windows
        } else {
            Os::Linux
        };
        let arch = if cfg!(target_arch = "aarch64") {
            Arch::Arm64
        } else if cfg!(target_pointer_width = "32") {
            Arch::X86
        } else {
            Arch::X64
        };
        Platform { os, arch }
    }
}

impl ToolId {
    pub fn file_name(self, os: Os) -> &'static str {
        match (self, os) {
            (ToolId::YtDlp, Os::Windows) => "yt-dlp.exe",
            (ToolId::YtDlp, _) => "yt-dlp",
            (ToolId::Ffmpeg, Os::Windows) => "ffmpeg.exe",
            (ToolId::Ffmpeg, _) => "ffmpeg",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            ToolId::YtDlp => "yt-dlp",
            ToolId::Ffmpeg => "ffmpeg",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsStatus {
    pub ytdlp: Option<PathBuf>,
    pub ffmpeg: Option<PathBuf>,
}

/// Find a tool: first on the given PATH, then in the app's own tools folder.
pub fn locate_in(tool: ToolId, managed_dir: &Path, path_var: Option<&OsStr>) -> Option<PathBuf> {
    let name = tool.file_name(Platform::current().os);
    if let Some(var) = path_var {
        for dir in std::env::split_paths(var) {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    let managed = managed_dir.join(name);
    managed.is_file().then_some(managed)
}

/// PATH plus the folders GUI apps on macOS do not inherit (Homebrew).
pub fn search_path() -> OsString {
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|v| std::env::split_paths(&v).collect())
        .unwrap_or_default();
    if cfg!(target_os = "macos") {
        dirs.push(PathBuf::from("/opt/homebrew/bin"));
        dirs.push(PathBuf::from("/usr/local/bin"));
    }
    std::env::join_paths(dirs).unwrap_or_default()
}

pub fn locate(tool: ToolId, managed_dir: &Path) -> Option<PathBuf> {
    locate_in(tool, managed_dir, Some(search_path().as_os_str()))
}

pub fn status_in(managed_dir: &Path, path_var: Option<&OsStr>) -> ToolsStatus {
    ToolsStatus {
        ytdlp: locate_in(ToolId::YtDlp, managed_dir, path_var),
        ffmpeg: locate_in(ToolId::Ffmpeg, managed_dir, path_var),
    }
}

pub fn status(managed_dir: &Path) -> ToolsStatus {
    status_in(managed_dir, Some(search_path().as_os_str()))
}

/// What to tell the user when automatic installation is impossible.
pub fn manual_hint(tool: ToolId, p: Platform) -> String {
    let name = tool.display_name();
    match p.os {
        Os::MacOs => format!("brew install {name}"),
        Os::Windows => format!("winget install {name}"),
        Os::Linux => format!("sudo apt install {name}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(os: Os, arch: Arch) -> Platform {
        Platform { os, arch }
    }

    #[test]
    fn locate_prefers_path_then_managed() {
        let path_dir = tempfile::tempdir().unwrap();
        let managed = tempfile::tempdir().unwrap();
        let os = Platform::current().os;
        let name = ToolId::YtDlp.file_name(os);
        assert_eq!(locate_in(ToolId::YtDlp, managed.path(), Some(path_dir.path().as_os_str())), None);
        std::fs::write(managed.path().join(name), b"x").unwrap();
        assert_eq!(
            locate_in(ToolId::YtDlp, managed.path(), Some(path_dir.path().as_os_str())),
            Some(managed.path().join(name))
        );
        std::fs::write(path_dir.path().join(name), b"x").unwrap();
        assert_eq!(
            locate_in(ToolId::YtDlp, managed.path(), Some(path_dir.path().as_os_str())),
            Some(path_dir.path().join(name))
        );
    }

    #[test]
    fn manual_hint_per_os() {
        assert!(manual_hint(ToolId::Ffmpeg, p(Os::MacOs, Arch::Arm64)).contains("brew install ffmpeg"));
        assert!(manual_hint(ToolId::YtDlp, p(Os::Windows, Arch::X86)).contains("winget install yt-dlp"));
        assert!(manual_hint(ToolId::Ffmpeg, p(Os::Linux, Arch::X64)).contains("apt install ffmpeg"));
    }

    #[test]
    fn file_names_follow_os() {
        assert_eq!(ToolId::YtDlp.file_name(Os::Windows), "yt-dlp.exe");
        assert_eq!(ToolId::Ffmpeg.file_name(Os::MacOs), "ffmpeg");
    }

    #[test]
    fn status_reports_missing_tools_as_none() {
        let managed = tempfile::tempdir().unwrap();
        let s = status_in(managed.path(), Some(managed.path().as_os_str()));
        assert_eq!((s.ytdlp, s.ffmpeg), (None, None));
    }
}
