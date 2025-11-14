# Implementation Plan

- [ ] 1. Update project dependencies for modern macOS bindings

  - Replace deprecated cocoa crate with objc2-app-kit and core-foundation
  - Update Cargo.toml to use objc2 framework (objc2, objc2-app-kit, objc2-foundation)
  - Remove cocoa and objc dependencies
  - Update imports in spotify_client.rs to use objc2 for notifications
  - _Requirements: 6.3_

- [x] 2. Implement Spotify authentication and credential management

  - [x] 2.1 Create SpotifyClient struct with rspotify AuthCodeSpotify client
    - Implement OAuth2 authorization code flow using rspotify
    - Configure required scopes: user-read-currently-playing, user-read-playback-state
    - _Requirements: 1.1_
  - [x] 2.2 Implement secure token storage in macOS keychain
    - Use keyring-rs to store access token, refresh token, and expiry
    - Implement token retrieval and validation on app startup
    - _Requirements: 1.2_
  - [x] 2.3 Implement automatic token refresh logic
    - Detect expired tokens and trigger refresh using rspotify
    - Handle refresh failures and prompt re-authentication
    - _Requirements: 1.3_
  - [x] 2.4 Add authentication success notification
    - Display macOS notification when authentication completes
    - _Requirements: 1.4_

- [x] 3. Implement Spotify track polling

  - [x] 3.1 Create TrackInfo data structure
    - Define struct with id, name, artists, and duration_ms fields
    - Implement conversion from rspotify's FullTrack type
    - _Requirements: 2.3_
  - [x] 3.2 Implement async polling loop with tokio
    - Create tokio task that polls every 5 seconds using tokio::time::interval
    - Use rspotify's current_playing endpoint to get track info
    - _Requirements: 2.1, 6.2_
  - [x] 3.3 Implement track change detection
    - Compare current track ID with previous track ID
    - Send TrackUpdate event via mpsc channel when track changes
    - _Requirements: 2.2_
  - [x] 3.4 Add error handling with exponential backoff
    - Implement retry logic (3 attempts) with delays of 1s, 2s, 4s
    - Log errors and continue polling after failures
    - _Requirements: 2.4_

- [x] 4. Implement lyrics fetching and caching

  - [x] 4.1 Create LyricsFetcher struct with HTTP client
    - Initialize reqwest client with appropriate timeouts (10 seconds)
    - Define Lyrics and CachedLyrics data structures
    - _Requirements: 3.2_
  - [x] 4.2 Implement Lyrics.ovh API integration
    - Create async function to query Lyrics.ovh API with artist and title
    - URL-encode artist and title for path parameters
    - Parse JSON response and extract lyrics field
    - Handle API errors gracefully (404 for not found, network errors)
    - _Requirements: 3.1, 3.2_
  - [x] 4.3 Implement in-memory LRU cache
    - Create HashMap-based cache with max 100 entries
    - Implement cache lookup by track ID
    - Implement LRU eviction when cache is full
    - _Requirements: 3.5_
  - [x] 4.4 Add lyrics unavailable handling
    - Return appropriate error when lyrics not found
    - Cache negative results to avoid repeated API calls
    - _Requirements: 3.4_

- [x] 5. Implement configuration management

  - [x] 5.1 Create AppConfig struct
    - Define fields: window_position, overlay_visible, poll_interval_secs
    - Implement serde serialization/deserialization
    - _Requirements: 4.5_
  - [x] 5.2 Implement config file persistence
    - Create config directory at ~/Library/Application Support/com.lyricsify/
    - Implement save_config and load_config functions using serde_json
    - Handle missing or corrupted config files with defaults
    - _Requirements: 4.5, 7.3_

- [x] 6. Implement macOS overlay window

  - [x] 6.1 Create OverlayWindow struct using objc2-app-kit
    - Initialize NSWindow with appropriate style mask using objc2 APIs
    - Set window level to NSFloatingWindowLevel for always-on-top
    - Configure window to be non-activating
    - Use Retained<NSWindow> for memory management
    - _Requirements: 4.3_
  - [x] 6.2 Configure window appearance
    - Set window opacity to 0.8 (80%)
    - Add NSVisualEffectView for blur background effect
    - Set default size to 400x600 pixels
    - Add rounded corners with 12px radius
    - _Requirements: 4.1, 4.6_
  - [x] 6.3 Add NSTextView for lyrics display
    - Create scrollable text view with appropriate styling
    - Set font to SF Pro Text, 14pt, white color
    - Configure line spacing to 1.5 and padding to 20px
    - _Requirements: 4.6_
  - [x] 6.4 Implement window positioning
    - Set default position to top-right corner of screen using CGPoint
    - Make window draggable by implementing mouse event handlers with objc2
    - Load saved position from config on startup
    - Save position to config when window moves
    - _Requirements: 4.2, 4.4, 4.5_
  - [x] 6.5 Implement show/hide functionality
    - Add methods to show and hide window
    - Ensure window state persists in config
    - _Requirements: 5.3, 5.4_

- [x] 7. Implement menu bar integration

  - [x] 7.1 Create MenuBar struct with NSStatusItem using objc2
    - Initialize status item in system menu bar using objc2-app-kit
    - Set icon to musical note symbol (SF Symbols)
    - Use Retained<NSStatusItem> for memory management
    - _Requirements: 5.1_
  - [x] 7.2 Create dropdown menu with options
    - Add "Show Lyrics" / "Hide Lyrics" toggle menu item
    - Add "Authenticate Spotify" menu item (conditional on auth state)
    - Add "Quit" menu item
    - _Requirements: 5.2, 7.1_
  - [x] 7.3 Implement menu item actions
    - Connect toggle item to show/hide overlay functionality
    - Connect authenticate item to OAuth flow
    - Connect quit item to graceful shutdown
    - _Requirements: 5.3, 5.4, 7.2_
  - [x] 7.4 Implement visual state indicators
    - Change icon color based on overlay visibility (colored when visible, gray when hidden)
    - Update menu item text dynamically ("Show" vs "Hide")
    - _Requirements: 5.5_

- [ ] 8. Implement application core and event loop

  - [ ] 8.1 Create App struct coordinating all components
    - Initialize SpotifyClient, LyricsFetcher, and UIManager
    - Set up mpsc channels for event communication
    - Load configuration on startup
    - _Requirements: All_
  - [ ] 8.2 Implement main event loop with tokio
    - Use tokio::select! to handle multiple event sources
    - Handle TrackChanged events by fetching lyrics
    - Handle LyricsRetrieved events by updating UI
    - Handle ToggleOverlay events from menu bar
    - _Requirements: 2.2, 3.1_
  - [ ] 8.3 Implement graceful shutdown
    - Handle Quit events from menu bar
    - Save configuration before exit
    - Clean up background tasks and resources
    - _Requirements: 7.2, 7.3, 7.4_
  - [ ] 8.4 Wire up complete flow from track change to lyrics display
    - Ensure track polling triggers lyrics fetch
    - Ensure fetched lyrics update overlay window
    - Verify error states display appropriate messages
    - _Requirements: 2.2, 3.1, 3.3, 3.4_

- [ ] 9. Add error handling and user feedback

  - [ ] 9.1 Implement error display in overlay
    - Show "Unable to connect to Spotify" for API errors
    - Show "Lyrics not available" when lyrics not found
    - Show "Not authenticated" when credentials missing
    - _Requirements: 2.4, 3.4_

- [ ]\* 10. Performance optimization
  - [ ]\* 10.1 Implement debouncing for rapid track changes
    - Wait 1 second before fetching lyrics after track change
    - Cancel pending fetch if another track change occurs
    - _Requirements: 6.1, 6.2_
  - [ ]\* 10.2 Optimize UI updates
    - Only redraw overlay when lyrics content changes
    - Release window resources when overlay is hidden
    - _Requirements: 6.1, 6.4_
  - [ ]\* 10.3 Profile memory and CPU usage
    - Verify memory usage stays under 50 MB
    - Verify CPU usage is minimal during normal operation
    - _Requirements: 6.1, 6.3_
