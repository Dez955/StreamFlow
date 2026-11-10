use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Format {
    pub format_id: String,
    pub ext: String,
    pub vcodec: Option<String>,
    pub acodec: Option<String>,
    pub height: Option<u32>,
    pub tbr: Option<f64>,
    pub abr: Option<f64>,
    pub filesize: Option<u64>,
    pub filesize_approx: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoInfo {
    pub id: String,
    pub title: String,
    pub uploader: Option<String>,
    pub duration_secs: Option<u64>,
    pub thumbnail: Option<String>,
    pub webpage_url: String,
    pub is_playlist: bool,
    pub formats: Vec<Format>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoChoice {
    pub format_id: String,
    pub height: u32,
    pub codec_label: String,
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioChoice {
    pub format_id: String,
    pub abr: u32,
    pub codec_label: String,
    pub size_bytes: Option<u64>,
}

/// `None` means "no video" (audio-only download). Never glob-import this enum.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VideoSelect {
    Best,
    Format(String),
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AudioSelect {
    Best,
    Format(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioOnlyFormat {
    #[default]
    Mp3,
    M4a,
    Opus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Container {
    #[default]
    Mp4,
    Mkv,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRequest {
    pub url: String,
    pub title: String,
    pub thumbnail: Option<String>,
    pub video: VideoSelect,
    pub audio: AudioSelect,
    #[serde(default)]
    pub audio_only_format: AudioOnlyFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JobState {
    Queued,
    Downloading,
    Processing,
    Paused,
    Stopped,
    Done,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub downloaded: u64,
    pub total: Option<u64>,
    pub speed_bps: Option<f64>,
    pub eta_secs: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrips_as_camel_case() {
        let req = DownloadRequest {
            url: "https://example.com/v".into(),
            title: "T".into(),
            thumbnail: None,
            video: VideoSelect::Best,
            audio: AudioSelect::Best,
            audio_only_format: AudioOnlyFormat::default(),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"audioOnlyFormat\":\"mp3\""), "{json}");
        assert!(json.contains("\"video\":\"best\""), "{json}");
        let back: DownloadRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back, req);
        let explicit: VideoSelect = serde_json::from_str("{\"format\":\"137\"}").unwrap();
        assert_eq!(explicit, VideoSelect::Format("137".into()));
        assert_eq!(Container::default(), Container::Mp4);
    }
}
