mod app_core;
mod config;
mod error;
mod lyrics_fetcher;
mod spotify_client;
mod ui_manager;

use app_core::App;
use error::LyricsifyError;

#[tokio::main]
async fn main() -> Result<(), LyricsifyError> {
    // Initialize logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    log::info!("Starting Lyricsify...");

    // Create and initialize the application
    let mut app = App::new()?;
    log::info!("Application created successfully");

    // Initialize components (load config, authenticate, start polling)
    app.initialize().await?;
    log::info!("Application initialized successfully");

    // Run the main event loop
    app.run().await?;

    log::info!("Lyricsify shutdown complete");
    Ok(())
}
