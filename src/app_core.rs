use crate::error::LyricsifyError;
use crate::spotify_client::TrackInfo;

/// Events that can occur in the application
#[derive(Debug, Clone)]
pub enum AppEvent {
    TrackChanged(TrackInfo),
    LyricsRetrieved(Option<String>),
    ToggleOverlay,
    Authenticate,
    Quit,
    SpotifyError(String),
}

pub struct App {
    // Implementation will be added in subsequent tasks
}

impl App {
    pub fn new() -> Result<Self, LyricsifyError> {
        todo!("Implementation in task 8")
    }

    pub async fn run(&mut self) -> Result<(), LyricsifyError> {
        todo!("Implementation in task 8")
    }
}
