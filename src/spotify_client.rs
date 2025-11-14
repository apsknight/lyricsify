use crate::error::LyricsifyError;
use crate::app_core::AppEvent;
use rspotify::{
    clients::OAuthClient,
    model::PlayableItem,
    AuthCodeSpotify, Config, Credentials, OAuth, Token,
};
use rspotify::scopes;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tokio::time::{interval, Duration};
use keyring::Entry;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
// Notification support will be added in menu bar implementation

/// Information about a Spotify track
#[derive(Debug, Clone, PartialEq)]
pub struct TrackInfo {
    pub id: String,
    pub name: String,
    pub artists: Vec<String>,
    pub duration_ms: u64,
}

impl TrackInfo {
    /// Convert from rspotify's FullTrack type
    pub fn from_full_track(track: &rspotify::model::FullTrack) -> Self {
        Self {
            id: track.id.as_ref().map(|id| id.to_string()).unwrap_or_default(),
            name: track.name.clone(),
            artists: track.artists.iter().map(|a| a.name.clone()).collect(),
            duration_ms: track.duration.num_milliseconds() as u64,
        }
    }
}

/// Serializable token data for keychain storage
#[derive(Debug, Serialize, Deserialize)]
struct StoredToken {
    access_token: String,
    refresh_token: Option<String>,
    expires_at: Option<DateTime<Utc>>,
    scopes: Vec<String>,
}

const KEYCHAIN_SERVICE: &str = "com.lyricsify.spotify";
const KEYCHAIN_ACCOUNT: &str = "spotify_token";

/// Display a macOS notification
/// 
/// This function uses the native NSUserNotificationCenter API to display
/// a notification to the user.
fn show_notification(title: &str, message: &str) {
    // For now, just log the notification
    // Full notification support will be added when implementing menu bar
    log::info!("Notification: {} - {}", title, message);
}

/// Manages Spotify authentication and API interactions
pub struct SpotifyClient {
    client: Arc<AuthCodeSpotify>,
    current_track: Arc<Mutex<Option<TrackInfo>>>,
}

impl SpotifyClient {
    /// Create a new SpotifyClient with OAuth2 configuration
    /// 
    /// This initializes the client with the required scopes for reading
    /// currently playing track information.
    pub fn new() -> Result<Self, LyricsifyError> {
        // Get credentials from environment variables
        let client_id = std::env::var("SPOTIFY_CLIENT_ID")
            .map_err(|_| LyricsifyError::AuthenticationFailed(
                "SPOTIFY_CLIENT_ID environment variable not set".to_string()
            ))?;
        
        let client_secret = std::env::var("SPOTIFY_CLIENT_SECRET")
            .map_err(|_| LyricsifyError::AuthenticationFailed(
                "SPOTIFY_CLIENT_SECRET environment variable not set".to_string()
            ))?;

        let redirect_uri = std::env::var("SPOTIFY_REDIRECT_URI")
            .unwrap_or_else(|_| "http://localhost:8888/callback".to_string());

        // Set up credentials
        let creds = Credentials::new(&client_id, &client_secret);

        // Configure OAuth with required scopes
        let oauth = OAuth {
            redirect_uri,
            scopes: scopes!(
                "user-read-currently-playing",
                "user-read-playback-state"
            ),
            ..Default::default()
        };

        // Create the Spotify client with configuration
        let config = Config {
            token_cached: true,
            token_refreshing: true,
            ..Default::default()
        };

        let client = AuthCodeSpotify::with_config(creds, oauth, config);

        Ok(Self {
            client: Arc::new(client),
            current_track: Arc::new(Mutex::new(None)),
        })
    }

    /// Initiate the OAuth2 authorization flow
    /// 
    /// This generates the authorization URL that the user needs to visit
    /// to grant permissions to the application.
    pub fn get_auth_url(&self) -> Result<String, LyricsifyError> {
        let url = self.client.get_authorize_url(false)
            .map_err(|e| LyricsifyError::AuthenticationFailed(
                format!("Failed to generate auth URL: {}", e)
            ))?;
        Ok(url)
    }

    /// Complete the OAuth2 flow by exchanging the authorization code for tokens
    /// 
    /// After the user authorizes the application, Spotify redirects to the
    /// redirect_uri with a code parameter. This method exchanges that code
    /// for access and refresh tokens.
    pub async fn authenticate_with_code(&self, code: &str) -> Result<(), LyricsifyError> {
        self.client.request_token(code).await
            .map_err(|e| LyricsifyError::AuthenticationFailed(
                format!("Failed to exchange code for token: {}", e)
            ))?;
        
        log::info!("Successfully authenticated with Spotify");
        
        // Display success notification
        show_notification(
            "Lyricsify",
            "Successfully authenticated with Spotify!"
        );
        
        Ok(())
    }

    /// Set the token directly (used when loading from keychain)
    pub async fn set_token(&self, token: Token) -> Result<(), LyricsifyError> {
        *self.client.token.lock().await.unwrap() = Some(token);
        log::info!("Token set successfully");
        Ok(())
    }

    /// Get the current token (for storage in keychain)
    pub async fn get_token(&self) -> Result<Option<Token>, LyricsifyError> {
        let token = self.client.token.lock().await.unwrap().clone();
        Ok(token)
    }

    /// Check if the client is currently authenticated
    pub async fn is_authenticated(&self) -> bool {
        self.client.token.lock().await.unwrap().is_some()
    }

    /// Get the currently playing track from Spotify
    pub async fn get_current_track(&self) -> Result<Option<TrackInfo>, LyricsifyError> {
        let currently_playing = self.client
            .current_playing(None, None::<Vec<_>>)
            .await
            .map_err(|e| LyricsifyError::SpotifyApiError(
                format!("Failed to get currently playing track: {}", e)
            ))?;

        if let Some(playing) = currently_playing {
            if let Some(item) = playing.item {
                match item {
                    PlayableItem::Track(track) => {
                        let track_info = TrackInfo::from_full_track(&track);
                        return Ok(Some(track_info));
                    }
                    PlayableItem::Episode(_) => {
                        // We don't support podcasts for lyrics
                        return Ok(None);
                    }
                }
            }
        }

        Ok(None)
    }

    /// Get a reference to the internal client for advanced operations
    pub fn client(&self) -> Arc<AuthCodeSpotify> {
        Arc::clone(&self.client)
    }

    /// Save the current token to the macOS keychain
    /// 
    /// This securely stores the access token, refresh token, and expiry
    /// information in the system keychain for persistence across app restarts.
    pub async fn save_token_to_keychain(&self) -> Result<(), LyricsifyError> {
        let token = self.get_token().await?;
        
        if let Some(token) = token {
            let stored_token = StoredToken {
                access_token: token.access_token,
                refresh_token: token.refresh_token,
                expires_at: token.expires_at,
                scopes: token.scopes.into_iter().collect(),
            };

            let json = serde_json::to_string(&stored_token)?;
            
            let entry = Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)?;
            entry.set_password(&json)?;
            
            log::info!("Token saved to keychain successfully");
            Ok(())
        } else {
            Err(LyricsifyError::AuthenticationFailed(
                "No token available to save".to_string()
            ))
        }
    }

    /// Load token from the macOS keychain
    /// 
    /// Retrieves the stored token from the keychain and sets it in the client.
    /// Returns true if a valid token was loaded, false if no token exists.
    pub async fn load_token_from_keychain(&self) -> Result<bool, LyricsifyError> {
        let entry = Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)?;
        
        match entry.get_password() {
            Ok(json) => {
                let stored_token: StoredToken = serde_json::from_str(&json)?;
                
                // Convert back to rspotify Token
                let token = Token {
                    access_token: stored_token.access_token,
                    refresh_token: stored_token.refresh_token,
                    expires_at: stored_token.expires_at,
                    scopes: stored_token.scopes.into_iter().collect(),
                    expires_in: chrono::Duration::zero(), // Not used when expires_at is set
                };

                self.set_token(token).await?;
                log::info!("Token loaded from keychain successfully");
                Ok(true)
            }
            Err(keyring::Error::NoEntry) => {
                log::info!("No token found in keychain");
                Ok(false)
            }
            Err(e) => Err(LyricsifyError::KeyringError(e)),
        }
    }

    /// Validate that the current token is not expired
    /// 
    /// Returns true if the token exists and is still valid, false otherwise.
    pub async fn is_token_valid(&self) -> bool {
        if let Ok(Some(token)) = self.get_token().await {
            if let Some(expires_at) = token.expires_at {
                // Consider token valid if it expires more than 60 seconds from now
                let now = Utc::now();
                let buffer = chrono::Duration::seconds(60);
                return expires_at > now + buffer;
            }
        }
        false
    }

    /// Clear the token from the keychain
    /// 
    /// Useful for logout or when re-authentication is required.
    pub fn clear_token_from_keychain(&self) -> Result<(), LyricsifyError> {
        let entry = Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)?;
        
        match entry.delete_credential() {
            Ok(_) => {
                log::info!("Token cleared from keychain");
                Ok(())
            }
            Err(keyring::Error::NoEntry) => {
                // Already cleared, not an error
                Ok(())
            }
            Err(e) => Err(LyricsifyError::KeyringError(e)),
        }
    }

    /// Refresh the access token using the refresh token
    /// 
    /// This method attempts to refresh the access token when it's expired or
    /// about to expire. If the refresh fails, it clears the stored token and
    /// returns an error indicating re-authentication is needed.
    pub async fn refresh_token(&self) -> Result<(), LyricsifyError> {
        log::info!("Attempting to refresh token");
        
        // Check if we have a token to refresh
        if !self.is_authenticated().await {
            return Err(LyricsifyError::AuthenticationFailed(
                "No token available to refresh".to_string()
            ));
        }

        // Attempt to refresh the token
        // rspotify automatically refreshes tokens when token_refreshing is enabled
        // We just need to trigger a request that will cause the refresh
        match self.client.current_user().await {
            Ok(_) => {
                log::info!("Token refreshed successfully");
                
                // Save the new token to keychain
                self.save_token_to_keychain().await?;
                Ok(())
            }
            Err(e) => {
                log::error!("Token refresh failed: {}", e);
                
                // Clear the invalid token from keychain
                let _ = self.clear_token_from_keychain();
                
                Err(LyricsifyError::AuthenticationFailed(
                    format!("Token refresh failed, re-authentication required: {}", e)
                ))
            }
        }
    }

    /// Ensure the token is valid, refreshing if necessary
    /// 
    /// This is a convenience method that checks token validity and automatically
    /// refreshes it if needed. Should be called before making API requests.
    pub async fn ensure_token_valid(&self) -> Result<(), LyricsifyError> {
        if !self.is_authenticated().await {
            return Err(LyricsifyError::AuthenticationFailed(
                "Not authenticated".to_string()
            ));
        }

        if !self.is_token_valid().await {
            log::info!("Token expired or about to expire, refreshing");
            self.refresh_token().await?;
        }

        Ok(())
    }

    /// Initialize the client by loading token from keychain and validating it
    /// 
    /// This should be called on app startup. It will:
    /// 1. Try to load token from keychain
    /// 2. Validate the token
    /// 3. Refresh if expired
    /// 4. Return whether authentication is needed
    /// 
    /// Returns Ok(true) if authenticated, Ok(false) if authentication needed
    pub async fn initialize(&self) -> Result<bool, LyricsifyError> {
        log::info!("Initializing Spotify client");
        
        // Try to load token from keychain
        let token_loaded = self.load_token_from_keychain().await?;
        
        if !token_loaded {
            log::info!("No stored token found, authentication required");
            return Ok(false);
        }

        // Check if token is valid
        if self.is_token_valid().await {
            log::info!("Stored token is valid");
            return Ok(true);
        }

        // Token is expired, try to refresh
        log::info!("Stored token is expired, attempting refresh");
        match self.refresh_token().await {
            Ok(_) => {
                log::info!("Token refreshed successfully");
                Ok(true)
            }
            Err(_) => {
                log::warn!("Token refresh failed, authentication required");
                Ok(false)
            }
        }
    }

    /// Start polling for track changes
    /// 
    /// This creates a background task that polls the Spotify API every 5 seconds
    /// to check for track changes. When a track change is detected, it sends a
    /// TrackChanged event through the provided channel.
    /// 
    /// The polling loop includes error handling with exponential backoff and
    /// continues running even after errors.
    pub fn start_polling(&self, event_tx: mpsc::Sender<AppEvent>) {
        let client = Arc::clone(&self.client);
        let current_track = Arc::clone(&self.current_track);
        
        tokio::spawn(async move {
            let mut poll_interval = interval(Duration::from_secs(5));
            log::info!("Started Spotify track polling (5 second interval)");
            
            loop {
                poll_interval.tick().await;
                
                // Attempt to get current track with retry logic
                match Self::get_current_track_with_retry(&client).await {
                    Ok(new_track) => {
                        // Check if track has changed
                        let mut current = current_track.lock().await;
                        
                        if *current != new_track {
                            log::info!("Track changed: {:?}", new_track);
                            
                            // Update stored track
                            *current = new_track.clone();
                            
                            // Send event if track exists
                            if let Some(track) = new_track {
                                if let Err(e) = event_tx.send(AppEvent::TrackChanged(track)).await {
                                    log::error!("Failed to send TrackChanged event: {}", e);
                                    break; // Exit if channel is closed
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to get current track after retries: {}", e);
                        
                        // Send error event
                        if let Err(send_err) = event_tx.send(AppEvent::SpotifyError(e.to_string())).await {
                            log::error!("Failed to send SpotifyError event: {}", send_err);
                            break; // Exit if channel is closed
                        }
                    }
                }
            }
            
            log::warn!("Spotify polling loop terminated");
        });
    }

    /// Get current track with exponential backoff retry logic
    /// 
    /// Attempts to fetch the current track up to 3 times with delays of 1s, 2s, 4s
    /// between attempts. Returns the track info or an error if all attempts fail.
    async fn get_current_track_with_retry(
        client: &AuthCodeSpotify,
    ) -> Result<Option<TrackInfo>, LyricsifyError> {
        let retry_delays = [1, 2, 4]; // Exponential backoff: 1s, 2s, 4s
        let mut last_error = None;
        
        for (attempt, &delay_secs) in retry_delays.iter().enumerate() {
            match client.current_playing(None, None::<Vec<_>>).await {
                Ok(currently_playing) => {
                    if let Some(playing) = currently_playing {
                        if let Some(item) = playing.item {
                            match item {
                                PlayableItem::Track(track) => {
                                    let track_info = TrackInfo::from_full_track(&track);
                                    return Ok(Some(track_info));
                                }
                                PlayableItem::Episode(_) => {
                                    // We don't support podcasts for lyrics
                                    return Ok(None);
                                }
                            }
                        }
                    }
                    return Ok(None);
                }
                Err(e) => {
                    log::warn!(
                        "Attempt {} failed to get current track: {}",
                        attempt + 1,
                        e
                    );
                    last_error = Some(e);
                    
                    // Don't sleep after the last attempt
                    if attempt < retry_delays.len() - 1 {
                        tokio::time::sleep(Duration::from_secs(delay_secs)).await;
                    }
                }
            }
        }
        
        Err(LyricsifyError::SpotifyApiError(format!(
            "Failed to get current track after {} attempts: {}",
            retry_delays.len(),
            last_error.map(|e| e.to_string()).unwrap_or_else(|| "Unknown error".to_string())
        )))
    }
}
