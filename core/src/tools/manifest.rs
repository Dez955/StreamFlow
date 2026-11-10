use super::{Arch, Os, Platform, ToolId};

#[derive(Debug, Clone, PartialEq)]
pub enum Sha256Source {
    /// Hash pinned in this file.
    Fixed(String),
    /// Hash looked up by file name in a `SHA2-256SUMS`-style text file.
    SumsFile { url: String, name: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolSource {
    pub url: String,
    pub sha256: Sha256Source,
    /// The download is a gzip of the executable.
    pub gunzip: bool,
}

const YTDLP_BASE: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download";
const FFMPEG_BASE: &str = "https://github.com/eugeneware/ffmpeg-static/releases/download";

fn ytdlp(p: Platform) -> Option<ToolSource> {
    let asset = match (p.os, p.arch) {
        (Os::MacOs, _) => "yt-dlp_macos",
        (Os::Windows, Arch::X64) => "yt-dlp.exe",
        (Os::Windows, Arch::X86) => "yt-dlp_x86.exe",
        (Os::Windows, Arch::Arm64) => "yt-dlp_arm64.exe",
        (Os::Linux, Arch::X64) => "yt-dlp_linux",
        (Os::Linux, Arch::Arm64) => "yt-dlp_linux_aarch64",
        (Os::Linux, Arch::X86) => return None,
    };
    Some(ToolSource {
        url: format!("{YTDLP_BASE}/{asset}"),
        sha256: Sha256Source::SumsFile { url: format!("{YTDLP_BASE}/SHA2-256SUMS"), name: asset.into() },
        gunzip: false,
    })
}

/// ffmpeg-static builds. Hashes are of the `.gz` files and were computed when pinned.
/// The 32-bit Windows build only exists up to tag b6.0.
fn ffmpeg(p: Platform) -> Option<ToolSource> {
    let (tag, asset, sha) = match (p.os, p.arch) {
        (Os::MacOs, Arch::Arm64) => ("b6.1.1", "ffmpeg-darwin-arm64.gz", "8923876afa8db5585022d7860ec7e589af192f441c56793971276d450ed3bbfa"),
        (Os::Windows, Arch::X64) => ("b6.1.1", "ffmpeg-win32-x64.gz", "8883a3dffbd0a16cf4ef95206ea05283f78908dbfb118f73c83f4951dcc06d77"),
        (Os::Windows, Arch::X86) => ("b6.0", "ffmpeg-win32-ia32.gz", "af47ca5de7b9f859f48e4e96ea831bf551b100144b0b871762ab48fba1152f56"),
        (Os::Linux, Arch::X64) => ("b6.1.1", "ffmpeg-linux-x64.gz", "bfe8a8fc511530457b528c48d77b5737527b504a3797a9bc4866aeca69c2dffa"),
        _ => return None,
    };
    Some(ToolSource {
        url: format!("{FFMPEG_BASE}/{tag}/{asset}"),
        sha256: Sha256Source::Fixed(sha.into()),
        gunzip: true,
    })
}

/// Where to download `tool` for `p`, or `None` when nothing is pinned for it.
pub fn source(tool: ToolId, p: Platform) -> Option<ToolSource> {
    match tool {
        ToolId::YtDlp => ytdlp(p),
        ToolId::Ffmpeg => ffmpeg(p),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::{Arch, Os, Platform, ToolId};

    fn p(os: Os, arch: Arch) -> Platform {
        Platform { os, arch }
    }

    #[test]
    fn manifest_covers_primary_platforms() {
        for plat in [p(Os::MacOs, Arch::Arm64), p(Os::Windows, Arch::X64), p(Os::Linux, Arch::X64)] {
            assert!(source(ToolId::YtDlp, plat).is_some(), "yt-dlp {plat:?}");
            assert!(source(ToolId::Ffmpeg, plat).is_some(), "ffmpeg {plat:?}");
        }
    }

    #[test]
    fn windows_32_bit_is_supported() {
        let y = source(ToolId::YtDlp, p(Os::Windows, Arch::X86)).unwrap();
        assert!(y.url.ends_with("/yt-dlp_x86.exe"), "{}", y.url);
        let f = source(ToolId::Ffmpeg, p(Os::Windows, Arch::X86)).unwrap();
        assert!(f.url.ends_with("ffmpeg-win32-ia32.gz"), "{}", f.url);
        assert!(f.gunzip);
    }

    #[test]
    fn ytdlp_hash_comes_from_sums_file_and_ffmpeg_hash_is_pinned() {
        let y = source(ToolId::YtDlp, p(Os::MacOs, Arch::Arm64)).unwrap();
        assert!(matches!(y.sha256, Sha256Source::SumsFile { ref name, .. } if name == "yt-dlp_macos"));
        let f = source(ToolId::Ffmpeg, p(Os::MacOs, Arch::Arm64)).unwrap();
        assert!(matches!(f.sha256, Sha256Source::Fixed(ref h) if h.len() == 64));
    }

    #[test]
    fn unpinned_platform_has_no_source() {
        assert!(source(ToolId::Ffmpeg, p(Os::MacOs, Arch::X64)).is_none());
    }
}
