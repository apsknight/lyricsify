use crate::config::AppConfig;
use crate::error::LyricsifyError;
use crate::lyrics_fetcher::LyricsFetcher;
use crate::spotify_client::{SpotifyClient, TrackInfo};
use crate::ui_manager::{MenuBar, UIManager};
use tokio::sync::mpsc;

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

/// Main application structure coordinating all components
pub struct App {
    spotify_client: SpotifyClient,
    lyrics_fetcher: LyricsFetcher,
    ui_manager: UIManager,
    menu_bar: MenuBar,
    config: AppConfig,
    event_rx: mpsc::Receiver<AppEvent>,
    event_tx: mpsc::Sender<AppEvent>,
}

impl App {
    /// Create a new App instance, initializing all components
    pub fn new() -> Result<Self, LyricsifyError> {
        log::info!("Initializing application components");

        // Load configuration from disk
        let config = AppConfig::load()?;
        log::info!("Configuration loaded");

        // Create event channel for communication between components
        let (event_tx, event_rx) = mpsc::channel(100);

        // Create unbounded channel for menu bar (UI events need to be non-blocking)
        let (menu_event_tx, mut menu_event_rx) = mpsc::unbounded_channel();

        // Initialize Spotify client
        let spotify_client = SpotifyClient::new()?;
        log::info!("Spotify client initialized");

        // Initialize lyrics fetcher
        let lyrics_fetcher = LyricsFetcher::new()?;
        log::info!("Lyrics fetcher initialized");

        // Initialize UI manager with overlay window
        let ui_manager = UIManager::new(config.clone())?;
        log::info!("UI manager initialized");

        // Initialize menu bar
        let menu_bar = MenuBar::new(menu_event_tx)?;
        log::info!("Menu bar initialized");

        // Spawn a task to forward menu events to the main event channel
        let event_tx_clone = event_tx.clone();
        tokio::spawn(async move {
            while let Some(event) = menu_event_rx.recv().await {
                if event_tx_clone.send(event).await.is_err() {
                    break;
                }
            }
        });

        Ok(Self {
            spotify_client,
            lyrics_fetcher,
            ui_manager,
            menu_bar,
            config,
            event_rx,
            event_tx,
        })
    }

    /// Initialize the application by setting up authentication and starting polling
    pub async fn initialize(&mut self) -> Result<(), LyricsifyError> {
        log::info!("Initializing application");

        // Initialize Spotify client (load token from keychain)
        let authenticated = self.spotify_client.initialize().await?;

        // Update menu bar authentication state
        self.menu_bar.update_auth_state(authenticated)?;

        if authenticated {
            log::info!("Authenticated with Spotify, starting track polling");
            // Start polling for track changes
            self.spotify_client.start_polling(self.event_tx.clone());
        } else {
            log::warn!("Not authenticated with Spotify. Please authenticate from the menu bar.");
        }

        // Update menu bar visibility state based on config
        self.menu_bar
            .update_visibility_state(self.config.overlay_visible)?;

        Ok(())
    }

    /// Run the main event loop
    pub async fn run(&mut self) -> Result<(), LyricsifyError> {
        log::info!("Starting main event loop");

        loop {
            tokio::select! {
                Some(event) = self.event_rx.recv() => {
                    match event {
                        AppEvent::TrackChanged(track) => {
                            self.handle_track_changed(track).await?;
                        }
                        AppEvent::LyricsRetrieved(lyrics) => {
                            self.handle_lyrics_retrieved(lyrics)?;
                        }
                        AppEvent::ToggleOverlay => {
                            self.handle_toggle_overlay()?;
                        }
                        AppEvent::Authenticate => {
                            self.handle_authenticate().await?;
                        }
                        AppEvent::Quit => {
                            log::info!("Quit event received");
                            self.shutdown()?;
                            break;
                        }
                        AppEvent::SpotifyError(error) => {
                            self.handle_spotify_error(error)?;
                        }
                    }
                }
                else => {
                    log::warn!("Event channel closed, exiting");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Handle track change event by fetching lyrics
    async fn handle_track_changed(&mut self, track: TrackInfo) -> Result<(), LyricsifyError> {
        log::info!(
            "Handling track change: {} by {}",
            track.name,
            track.artists.join(", ")
        );

        // Fetch lyrics for the new track
        let artist = track.artists.first().unwrap_or(&String::new()).clone();
        let lyrics = self
            .lyrics_fetcher
            .fetch_lyrics(&track.id, &artist, &track.name)
            .await?;

        // Send lyrics retrieved event
        self.event_tx
            .send(AppEvent::LyricsRetrieved(lyrics))
            .await
            .map_err(|e| {
                LyricsifyError::UIError(format!("Failed to send lyrics retrieved event: {}", e))
            })?;

        Ok(())
    }

    /// Handle lyrics retrieved event by updating the UI
    fn handle_lyrics_retrieved(&mut self, lyrics: Option<String>) -> Result<(), LyricsifyError> {
        if let Some(overlay) = self.ui_manager.overlay_window() {
            match lyrics {
                Some(text) => {
                    log::info!("Updating overlay with lyrics ({} chars)", text.len());
                    overlay.update_lyrics(&text)?;
                }
                None => {
                    log::info!("No lyrics available for this track");
                    overlay.update_lyrics("Lyrics not available for this track")?;
                }
            }
        }
        Ok(())
    }

    /// Handle toggle overlay event
    fn handle_toggle_overlay(&mut self) -> Result<(), LyricsifyError> {
        if let Some(overlay) = self.ui_manager.overlay_window() {
            let is_visible = overlay.is_visible();
            
            if is_visible {
                log::info!("Hiding overlay");
                overlay.hide()?;
                self.menu_bar.update_visibility_state(false)?;
            } else {
                log::info!("Showing overlay");
                overlay.show()?;
                self.menu_bar.update_visibility_state(true)?;
            }
        }
        Ok(())
    }

    /// Handle authenticate event
    async fn handle_authenticate(&mut self) -> Result<(), LyricsifyError> {
        log::info!("Starting authentication flow");

        // Get the authorization URL
        let auth_url = self.spotify_client.get_auth_url()?;
        
        log::info!("Please visit this URL to authenticate:");
        log::info!("{}", auth_url);
        
        // Open the URL in the default browser
        if let Err(e) = open_url(&auth_url) {
            log::error!("Failed to open browser: {}", e);
        }

        // In a real implementation, we would:
        // 1. Start a local HTTP server to receive the callback
        // 2. Wait for the authorization code
        // 3. Exchange it for tokens
        // 4. Save tokens to keychain
        // 5. Start polling
        //
        // For now, we'll just log the URL and expect manual handling
        log::warn!("Authentication flow requires manual completion");
        log::warn!("After authenticating, restart the application");

        Ok(())
    }

    /// Handle Spotify error event
    fn handle_spotify_error(&mut self, error: String) -> Result<(), LyricsifyError> {
        log::error!("Spotify error: {}", error);

        // Display error in overlay
        if let Some(overlay) = self.ui_manager.overlay_window() {
            overlay.update_lyrics(&format!("Unable to connect to Spotify\n\n{}", error))?;
        }

        Ok(())
    }

    /// Perform graceful shutdown
    fn shutdown(&mut self) -> Result<(), LyricsifyError> {
        log::info!("Shutting down application");

        // Save configuration
        self.config.save()?;
        log::info!("Configuration saved");

        // Clean up resources
        // (Tokio tasks will be automatically cancelled when the runtime shuts down)

        log::info!("Shutdown complete");
        Ok(())
    }
}

/// Open a URL in the default browser
fn open_url(url: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("Failed to open URL: {}", e))?;
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err("URL opening not supported on this platform".to_string())
    }
}
