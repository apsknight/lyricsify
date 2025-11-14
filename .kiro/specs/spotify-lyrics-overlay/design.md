# Design Document: Lyricsify

## Overview

Lyricsify is a native macOS application built in Rust that displays real-time synchronized lyrics for currently playing Spotify tracks. The application uses a translucent, always-on-top overlay window that allows users to view lyrics while multitasking. The system integrates with Spotify's Web API through the rspotify library and fetches lyrics from external sources.

### Key Design Goals

- **Non-intrusive**: Translucent overlay that doesn't obstruct workflow
- **Lightweight**: Minimal resource consumption (<50 MB memory)
- **Reliable**: Robust error handling and automatic recovery
- **Native**: macOS-first design using native APIs for optimal performance

## Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Menu Bar UI                          │
│                    (NSStatusItem)                           │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                     Application Core                        │
│                  (Event Loop & State)                       │
└─────┬──────────────────┬──────────────────┬─────────────────┘
      │                  │                  │
      ▼                  ▼                  ▼
┌──────────────┐  ┌──────────────┐  ┌──────────────────┐
│   Spotify    │  │   Lyrics     │  │  Overlay Window  │
│   Client     │  │   Fetcher    │  │   (NSWindow)     │
│  (rspotify)  │  │  (HTTP API)  │  │                  │
└──────┬───────┘  └──────┬───────┘  └──────────────────┘
       │                 │
       ▼                 ▼
┌──────────────┐  ┌──────────────┐
│  Spotify     │  │ Lyrics.ovh   │
│  Web API     │  │     API      │
└──────────────┘  └──────────────┘
```

### Component Interaction Flow

1. **Polling Loop**: Spotify Client polls API every 5 seconds
2. **Track Detection**: When track changes, event sent to Application Core
3. **Lyrics Fetch**: Application Core triggers Lyrics Fetcher
4. **Display Update**: Fetched lyrics sent to Overlay Window for display
5. **User Control**: Menu Bar UI sends show/hide/quit commands to Application Core

### Technology Stack

- **Language**: Rust (for performance, safety, and native integration)
- **Async Runtime**: Tokio (for concurrent operations)
- **Spotify Integration**: rspotify (Rust Spotify API wrapper)
- **HTTP Client**: reqwest (for lyrics API calls)
- **macOS UI**: core-foundation-rs + objc2 (native macOS bindings for AppKit/Cocoa)
- **Secure Storage**: keyring-rs (for OAuth token storage)
- **Serialization**: serde + serde_json (for config and API responses)
- **Error Handling**: thiserror (for structured error types)

## Components and Interfaces

### 1. Spotify Client Module

**Responsibility**: Manage Spotify authentication and track polling

#### SpotifyClient Struct

```rust
pub struct SpotifyClient {
    client: AuthCodeSpotify,
    current_track: Option<TrackInfo>,
    event_tx: mpsc::Sender<AppEvent>,
}
```

#### Key Methods

- `new(event_tx) -> Result<Self>`: Initialize with OAuth credentials
- `authenticate() -> Result<()>`: Execute OAuth flow and store tokens
- `start_polling()`: Begin async polling loop (5-second interval)
- `get_current_track() -> Result<Option<TrackInfo>>`: Query Spotify API
- `refresh_token() -> Result<()>`: Refresh expired OAuth token

#### TrackInfo Structure

```rust
pub struct TrackInfo {
    pub id: String,
    pub name: String,
    pub artists: Vec<String>,
    pub duration_ms: u64,
}
```

#### Design Decisions

- **Polling vs Webhooks**: Spotify doesn't provide webhooks for playback changes, so polling is necessary
- **5-second interval**: Balances responsiveness with API rate limits
- **Exponential backoff**: Prevents hammering API during network issues (1s, 2s, 4s delays)
- **Token storage**: macOS keychain provides secure, encrypted storage for OAuth tokens

### 2. Lyrics Fetcher Module

**Responsibility**: Retrieve and cache lyrics from external sources

#### LyricsFetcher Struct

```rust
pub struct LyricsFetcher {
    http_client: reqwest::Client,
    cache: LyricsCache,
}
```

#### LyricsCache Structure

```rust
struct LyricsCache {
    entries: HashMap<String, CachedLyrics>,
    access_order: VecDeque<String>,
    max_size: usize,
}

struct CachedLyrics {
    lyrics: Option<String>,
    timestamp: Instant,
}
```

#### Key Methods

- `new() -> Self`: Initialize with HTTP client and empty cache
- `fetch_lyrics(track: &TrackInfo) -> Result<Option<String>>`: Fetch from API or cache
- `query_lyrics_ovh(artist: &str, title: &str) -> Result<Option<String>>`: Query Lyrics.ovh API
- `cache_lyrics(track_id: &str, lyrics: Option<String>)`: Store in LRU cache

#### API Integration: Lyrics.ovh

- **Endpoint**: `GET https://api.lyrics.ovh/v1/{artist}/{title}`
- **Path params**: `artist`, `title` (URL-encoded)
- **Response**: JSON with `lyrics` field containing plain text lyrics
- **Timeout**: 10 seconds
- **Rate limiting**: Respectful caching to minimize requests
- **No authentication**: Public API, no API key required

#### Design Decisions

- **LRU cache**: 100-entry limit prevents unbounded memory growth
- **Negative caching**: Cache "not found" results to avoid repeated failed lookups
- **Single source initially**: Lyrics.ovh provides good coverage with simple API; fallback sources can be added later
- **In-memory cache**: Faster than disk, acceptable for 100 entries (~1-2 MB)

### 3. Overlay Window Module

**Responsibility**: Display lyrics in translucent, always-on-top window

#### OverlayWindow Struct

```rust
pub struct OverlayWindow {
    ns_window: Retained<NSWindow>,
    text_view: Retained<NSTextView>,
    current_position: CGPoint,
}
```

#### Window Configuration

- **Style**: Borderless, titled, closable, miniaturizable, resizable
- **Level**: `NSFloatingWindowLevel` (always-on-top)
- **Opacity**: 0.8 (80% translucent)
- **Background**: `NSVisualEffectView` with blur effect
- **Size**: 400x600 pixels (default)
- **Position**: Top-right corner (default), user-draggable

#### Text Display Configuration

- **Font**: SF Pro Text, 14pt, white color
- **Line spacing**: 1.5x for readability
- **Padding**: 20px on all sides
- **Scrolling**: Enabled for long lyrics
- **Alignment**: Left-aligned

#### Key Methods

- `new(config: &AppConfig) -> Result<Self>`: Create window with saved position
- `show()`: Make window visible
- `hide()`: Hide window (doesn't destroy)
- `update_lyrics(text: &str)`: Update displayed text
- `get_position() -> CGPoint`: Get current window position
- `set_position(point: CGPoint)`: Move window to position

#### Design Decisions

- **NSVisualEffectView**: Provides native macOS blur effect for modern appearance
- **Non-activating window**: Doesn't steal focus from other apps
- **Persistent positioning**: Remembers user's preferred location across sessions
- **Rounded corners**: 12px radius for polished appearance
- **objc2 framework**: Modern, safe Objective-C bindings replacing deprecated cocoa-rs

### 4. Menu Bar Module

**Responsibility**: Provide system tray interface for user controls

#### MenuBar Struct

```rust
pub struct MenuBar {
    status_item: Retained<NSStatusItem>,
    menu: Retained<NSMenu>,
    event_tx: mpsc::Sender<AppEvent>,
    overlay_visible: bool,
}
```

#### Menu Items

1. **Toggle Lyrics**: "Show Lyrics" / "Hide Lyrics" (dynamic text)
2. **Authenticate Spotify**: Visible only when not authenticated
3. **Quit**: Graceful shutdown

#### Icon States

- **Visible**: Colored musical note (SF Symbols: music.note)
- **Hidden**: Gray musical note
- **Not authenticated**: Warning icon

#### Key Methods

- `new(event_tx) -> Result<Self>`: Create status item and menu
- `update_visibility_state(visible: bool)`: Update icon and menu text
- `update_auth_state(authenticated: bool)`: Show/hide auth menu item

#### Design Decisions

- **SF Symbols**: Native macOS icons for consistent appearance
- **Visual feedback**: Icon color indicates overlay state at a glance
- **Minimal menu**: Only essential controls to avoid clutter

### 5. Application Core Module

**Responsibility**: Coordinate all components and manage application state

#### App Struct

```rust
pub struct App {
    spotify_client: SpotifyClient,
    lyrics_fetcher: LyricsFetcher,
    overlay_window: OverlayWindow,
    menu_bar: MenuBar,
    config: AppConfig,
    event_rx: mpsc::Receiver<AppEvent>,
}
```

#### AppEvent Enum

```rust
pub enum AppEvent {
    TrackChanged(TrackInfo),
    LyricsRetrieved(Option<String>),
    ToggleOverlay,
    Authenticate,
    Quit,
    SpotifyError(SpotifyError),
}
```

#### AppConfig Structure

```rust
pub struct AppConfig {
    pub window_position: (f64, f64),
    pub overlay_visible: bool,
    pub poll_interval_secs: u64,
}
```

#### Event Loop Flow

```rust
loop {
    tokio::select! {
        Some(event) = event_rx.recv() => {
            match event {
                TrackChanged(track) => fetch_lyrics(track),
                LyricsRetrieved(lyrics) => update_overlay(lyrics),
                ToggleOverlay => toggle_visibility(),
                Authenticate => start_auth_flow(),
                Quit => break,
                SpotifyError(err) => handle_error(err),
            }
        }
    }
}
```

#### Key Methods

- `new() -> Result<Self>`: Initialize all components
- `run() -> Result<()>`: Start event loop
- `handle_track_change(track: TrackInfo)`: Trigger lyrics fetch
- `handle_lyrics_retrieved(lyrics: Option<String>)`: Update UI
- `toggle_overlay_visibility()`: Show/hide window
- `shutdown()`: Save config and cleanup

#### Design Decisions

- **Event-driven architecture**: Decouples components, easier to test and maintain
- **Single event loop**: Centralized state management prevents race conditions
- **tokio::select!**: Efficient async event handling
- **Graceful shutdown**: Ensures config saved and resources cleaned up

## Data Models

### Configuration Persistence

**Location**: `~/Library/Application Support/com.lyricsify/config.json`

**Format**:

```json
{
  "window_position": [100.0, 100.0],
  "overlay_visible": true,
  "poll_interval_secs": 5
}
```

### OAuth Token Storage

**Location**: macOS Keychain
**Service**: "com.lyricsify.spotify"
**Account**: User's Spotify username

**Stored Data**:

- Access token
- Refresh token
- Token expiry timestamp

### Cache Structure

**In-memory only** (not persisted to disk)

```rust
HashMap<String, CachedLyrics>
// Key: Spotify track ID
// Value: Lyrics text (or None if unavailable) + timestamp
```

## Error Handling

### Error Types

```rust
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
}
```

### Error Recovery Strategies

| Error Type                | Recovery Strategy                                   |
| ------------------------- | --------------------------------------------------- |
| Token expired             | Automatic refresh, prompt re-auth if refresh fails  |
| Spotify API timeout       | Retry with exponential backoff (3 attempts)         |
| Lyrics not found          | Display "Lyrics unavailable", cache negative result |
| Network error             | Continue polling, display last known state          |
| Config load failure       | Use default configuration                           |
| UI initialization failure | Fatal error, log and exit gracefully                |

### User-Facing Error Messages

- **Spotify connection issues**: "Unable to connect to Spotify"
- **Authentication required**: "Please authenticate with Spotify"
- **Lyrics unavailable**: "Lyrics not available for this track"
- **Generic errors**: Logged but not displayed to avoid disruption

## Testing Strategy

### Unit Testing

**Target**: Individual component logic

- **SpotifyClient**: Mock rspotify responses, test track change detection
- **LyricsFetcher**: Mock HTTP responses, test cache behavior
- **AppConfig**: Test serialization/deserialization, default values
- **Error handling**: Test all error paths and recovery logic

### Integration Testing

**Target**: Component interactions

- **Auth flow**: Test OAuth flow end-to-end (with test credentials)
- **Track polling → Lyrics fetch**: Test event propagation
- **Config persistence**: Test save/load cycle
- **Cache behavior**: Test LRU eviction, negative caching

### Manual Testing

**Target**: UI and user experience

- **Window positioning**: Verify dragging, persistence across restarts
- **Menu bar interactions**: Test all menu items
- **Visual appearance**: Verify opacity, blur, font rendering
- **Performance**: Monitor memory and CPU usage during extended use

### Performance Testing

**Targets**:

- Memory usage < 50 MB during normal operation
- CPU usage < 5% when idle (overlay hidden)
- API polling doesn't cause UI lag
- Window updates are smooth (no flickering)

### Test Data

- **Mock tracks**: Create test TrackInfo instances
- **Mock lyrics**: Sample lyrics of varying lengths
- **Mock API responses**: JSON fixtures for Lyrics.ovh responses
- **Error scenarios**: Network timeouts, 404s, malformed JSON

## Performance Considerations

### Memory Optimization

- **Lyrics cache**: Limited to 100 entries (~1-2 MB)
- **String handling**: Use `Arc<String>` for shared lyrics text
- **Window resources**: Release when hidden to reduce memory footprint

### CPU Optimization

- **Polling interval**: 5 seconds balances responsiveness and CPU usage
- **Async operations**: Non-blocking I/O prevents UI freezes
- **Minimal redraws**: Only update window when lyrics actually change
- **Background throttling**: Reduce activity when overlay hidden

### Network Optimization

- **Caching**: Minimize redundant API calls
- **Connection pooling**: reqwest reuses HTTP connections
- **Timeouts**: 10-second timeout prevents hanging requests
- **Debouncing**: Wait 1 second after track change before fetching (handles rapid skipping)

## Security Considerations

### OAuth Token Security

- **Keychain storage**: Encrypted at rest by macOS
- **No token logging**: Tokens never written to logs
- **Secure transmission**: HTTPS only for all API calls

### API Key Management

- **No embedded keys**: Spotify client ID/secret from environment or config
- **User-specific tokens**: Each user authenticates with their own account

### Network Security

- **HTTPS only**: All external API calls use TLS
- **Certificate validation**: Default reqwest behavior validates certificates
- **No sensitive data in URLs**: Tokens in headers, not query params

## Future Enhancements

### Potential Features (Out of Scope for MVP)

- **Synchronized scrolling**: Auto-scroll lyrics based on playback position
- **Multiple lyrics sources**: Fallback to Genius, Musixmatch APIs
- **Customization**: Font size, color, opacity adjustments
- **Keyboard shortcuts**: Global hotkeys for show/hide
- **Multiple windows**: Support for multiple monitors
- **Lyrics editing**: Allow users to submit corrections
- **Translation**: Display lyrics in multiple languages

### Scalability Considerations

- **Disk cache**: For larger cache sizes, consider SQLite
- **Background sync**: Pre-fetch lyrics for playlist tracks
- **Cloud sync**: Sync preferences across devices via iCloud
