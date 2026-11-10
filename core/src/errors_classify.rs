use crate::error::{ErrorKind, UserError};

/// Map yt-dlp's stderr to a user-facing error kind. The original text is kept
/// in `detail` (last `ERROR:` line when present).
pub fn classify_stderr(stderr: &str) -> UserError {
    let text = stderr.trim();
    let low = text.to_lowercase();
    let has = |needles: &[&str]| needles.iter().any(|n| low.contains(n));

    let kind = if has(&["drm protected", "drm-protected", "drm protection"]) || low.contains("drm") && low.contains("protected") {
        ErrorKind::Drm
    } else if has(&["sign in", "log in", "login required", "registered users", "members-only", "age-restricted", "confirm your age"]) {
        ErrorKind::LoginRequired
    } else if has(&["unsupported url"]) {
        ErrorKind::Unsupported
    } else if has(&["no space left", "disk full", "not enough space"]) {
        ErrorKind::DiskFull
    } else if has(&[
        "unable to download webpage",
        "name resolution",
        "nodename nor servname",
        "network is unreachable",
        "connection refused",
        "timed out",
        "no route to host",
    ]) {
        ErrorKind::Network
    } else if has(&["unable to extract", "please report this issue", "update yt-dlp"]) {
        ErrorKind::NeedsUpdate
    } else {
        ErrorKind::Other
    };

    let detail = text
        .lines()
        .rev()
        .find(|l| l.trim_start().starts_with("ERROR:"))
        .unwrap_or(text);
    UserError::new(kind, detail.trim())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorKind;

    fn kind(s: &str) -> ErrorKind {
        classify_stderr(s).kind
    }

    #[test]
    fn classifies_known_failures() {
        assert_eq!(kind("ERROR: Unsupported URL: https://x.y"), ErrorKind::Unsupported);
        assert_eq!(kind("Sign in to confirm you're not a bot"), ErrorKind::LoginRequired);
        assert_eq!(kind("This video is only available for registered users"), ErrorKind::LoginRequired);
        assert_eq!(kind("ERROR: This video is DRM protected"), ErrorKind::Drm);
        assert_eq!(kind("OSError: [Errno 28] No space left on device"), ErrorKind::DiskFull);
        assert_eq!(
            kind("Unable to download webpage: <urlopen error [Errno 8] nodename nor servname provided>"),
            ErrorKind::Network
        );
        assert_eq!(kind("Temporary failure in name resolution"), ErrorKind::Network);
        assert_eq!(kind("ERROR: [site] abc: Unable to extract initial data"), ErrorKind::NeedsUpdate);
    }

    #[test]
    fn unknown_keeps_original_text() {
        let e = classify_stderr("something odd happened");
        assert_eq!(e.kind, ErrorKind::Other);
        assert!(e.detail.contains("something odd happened"));
    }

    #[test]
    fn drm_wins_over_unsupported_wording() {
        assert_eq!(kind("Unsupported: DRM protected content"), ErrorKind::Drm);
    }
}
