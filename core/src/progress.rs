use std::path::PathBuf;

use crate::args::{FILE_PREFIX, POST_PREFIX, PROGRESS_PREFIX};
use crate::model::Progress;

#[derive(Debug, Clone, PartialEq)]
pub enum LineEvent {
    Progress(Progress),
    PostProcess(String),
    File(PathBuf),
    Other,
}

fn num(s: &str) -> Option<f64> {
    s.trim().parse::<f64>().ok()
}

fn parse_progress(rest: &str) -> Option<Progress> {
    let parts: Vec<&str> = rest.split('|').collect();
    if parts.len() != 5 {
        return None;
    }
    Some(Progress {
        downloaded: num(parts[0])? as u64,
        total: num(parts[1]).or_else(|| num(parts[2])).map(|t| t as u64),
        speed_bps: num(parts[3]),
        eta_secs: num(parts[4]).map(|e| e as u64),
    })
}

/// Classify one line of yt-dlp stdout.
pub fn parse_line(line: &str) -> LineEvent {
    let line = line.trim_end_matches(['\r', '\n']);
    if let Some(rest) = line.strip_prefix(PROGRESS_PREFIX) {
        return parse_progress(rest).map_or(LineEvent::Other, LineEvent::Progress);
    }
    if let Some(rest) = line.strip_prefix(POST_PREFIX) {
        return LineEvent::PostProcess(rest.trim().to_string());
    }
    if let Some(rest) = line.strip_prefix(FILE_PREFIX) {
        return LineEvent::File(PathBuf::from(rest));
    }
    if let Some(rest) = line.strip_prefix("[download] ") {
        if let Some(path) = rest.strip_suffix(" has already been downloaded") {
            return LineEvent::File(PathBuf::from(path));
        }
    }
    LineEvent::Other
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn progress(line: &str) -> crate::model::Progress {
        match parse_line(line) {
            LineEvent::Progress(p) => p,
            other => panic!("expected Progress, got {other:?}"),
        }
    }

    #[test]
    fn parses_progress() {
        let p = progress("sf-progress:1048576|4194304|NA|524288.0|6");
        assert_eq!(p.downloaded, 1_048_576);
        assert_eq!(p.total, Some(4_194_304));
        assert_eq!(p.speed_bps, Some(524_288.0));
        assert_eq!(p.eta_secs, Some(6));
    }

    #[test]
    fn falls_back_to_estimate() {
        let p = progress("sf-progress:100|NA|2000|NA|NA");
        assert_eq!(p.total, Some(2000));
        assert_eq!(p.speed_bps, None);
        assert_eq!(p.eta_secs, None);
    }

    #[test]
    fn unknown_total_is_none() {
        assert_eq!(progress("sf-progress:100|NA|NA|NA|NA").total, None);
    }

    #[test]
    fn file_keeps_spaces_cyrillic_emoji() {
        let line = "sf-file:/Users/t/Downloads/Привіт світ 🎬.mp4";
        assert_eq!(
            parse_line(line),
            LineEvent::File(PathBuf::from("/Users/t/Downloads/Привіт світ 🎬.mp4"))
        );
    }

    #[test]
    fn already_downloaded_becomes_file() {
        assert_eq!(
            parse_line("[download] /tmp/My Clip.mp4 has already been downloaded"),
            LineEvent::File(PathBuf::from("/tmp/My Clip.mp4"))
        );
    }

    #[test]
    fn postprocess_and_other() {
        assert_eq!(parse_line("sf-post:Merger"), LineEvent::PostProcess("Merger".into()));
        assert_eq!(parse_line("[youtube] Extracting URL"), LineEvent::Other);
        assert_eq!(parse_line("sf-progress:garbage"), LineEvent::Other);
    }
}
