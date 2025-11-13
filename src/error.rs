use thiserror::Error;

#[derive(Error, Debug)]
pub enum LyricsifyError {
    #[error("Spotify authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Spotify API error: {0}")]
    SpotifyApiError(String),

    #[error("Lyrics fetch failed: {0}")]
    LyricsFetchError(String),

    #[error("UI error: {0}")]
    UIError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Keyring error: {0}")]
    KeyringError(#[from] keyring::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, LyricsifyError>;
