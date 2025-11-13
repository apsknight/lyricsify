mod app_core;
mod error;
mod lyrics_fetcher;
mod spotify_client;
mod ui_manager;

use error::LyricsifyError;

#[tokio::main]
async fn main() -> Result<(), LyricsifyError> {
    // Initialize logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    log::info!("Starting Lyricsify...");

    // Application initialization will be implemented in task 8
    log::info!("Lyricsify initialized successfully");

    Ok(())
}
