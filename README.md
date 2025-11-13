# Lyricsify

A macOS application that displays real-time lyrics for currently playing Spotify tracks as a translucent overlay. Perfect for multitaskers who want to view lyrics while working.

## Features

- **Real-time lyrics**: Automatically fetches and displays lyrics for your currently playing Spotify track
- **Non-intrusive overlay**: Translucent, always-on-top window that doesn't obstruct your workflow
- **Menu bar control**: Quick access to show/hide lyrics and manage the app
- **Lightweight**: Minimal resource consumption (<50 MB memory)
- **Native macOS**: Built with native APIs for optimal performance

## Status

🚧 **Early Development** - This project is currently in active development.

### Implemented Features

- ✅ Spotify OAuth2 authentication flow
- ✅ Secure token storage in macOS Keychain
- ✅ Automatic token refresh
- ✅ Track information retrieval from Spotify API
- ✅ Native macOS notifications for authentication success
- 🚧 Lyrics fetching (in progress)
- 🚧 Overlay window UI (in progress)
- 🚧 Menu bar integration (in progress)

## Technology Stack

- **Language**: Rust
- **Async Runtime**: Tokio
- **Spotify Integration**: rspotify
- **HTTP Client**: reqwest
- **macOS UI**: cocoa-rs
- **Secure Storage**: keyring-rs

## Dependencies

```toml
rspotify = { version = "0.13", features = ["client-reqwest"] }
tokio = { version = "1.40", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
keyring = "3.6"
cocoa = "0.26"
objc = "0.2"
thiserror = "2.0"
log = "0.4"
env_logger = "0.11"
```

## Building

```bash
# Debug build
cargo build

# Release build
cargo build --release
```

## Running

```bash
# Run debug version
cargo run

# Run release version
cargo run --release
```

## Development

```bash
# Check code without building
cargo check

# Run tests
cargo test

# Format code
cargo fmt

# Run linter
cargo clippy
```

## Architecture

The application is organized into modular components:

### Core Modules

- **spotify_client**: Handles Spotify authentication and track polling
  - OAuth2 authorization code flow
  - Secure token storage in macOS Keychain
  - Automatic token refresh
  - Currently playing track retrieval
  - Support for tracks (podcasts excluded)
- **lyrics_fetcher**: Retrieves and caches lyrics from external APIs
  - LRU cache for performance
  - Multiple lyrics source support
- **ui_manager**: Manages the overlay window and menu bar interface
  - Translucent overlay window
  - Menu bar status item
  - Window positioning and persistence
- **app_core**: Coordinates all components and manages application state
  - Event-driven architecture
  - Async event loop with Tokio
  - Configuration management
- **error**: Defines error types for the application
  - Structured error handling with thiserror
  - Authentication, API, and UI error types

### Data Flow

```
Spotify API → SpotifyClient → TrackInfo → LyricsFetcher → Lyrics → OverlayWindow
                    ↓
              macOS Keychain (Token Storage)
```

## Setup

### Prerequisites

- macOS (native macOS APIs used)
- Rust 1.70 or later
- Spotify account
- Spotify Developer Application credentials

### Spotify API Configuration

1. Create a Spotify application at [Spotify Developer Dashboard](https://developer.spotify.com/dashboard)
2. Note your Client ID and Client Secret
3. Add `http://localhost:8888/callback` to your app's Redirect URIs
4. Set the following environment variables:

```bash
export SPOTIFY_CLIENT_ID="your_client_id"
export SPOTIFY_CLIENT_SECRET="your_client_secret"
export SPOTIFY_REDIRECT_URI="http://localhost:8888/callback"  # Optional, defaults to this
```

### Authentication Flow

On first launch, the application will:

1. Generate an authorization URL
2. Open your browser for Spotify login
3. Request permissions for:
   - `user-read-currently-playing` - Read your currently playing track
   - `user-read-playback-state` - Read your playback state
4. Store the access token securely in macOS Keychain
5. Display a native macOS notification confirming successful authentication
6. Automatically refresh the token when it expires

The token is stored in the macOS Keychain under:

- **Service**: `com.lyricsify.spotify`
- **Account**: `spotify_token`

### Token Management

The application handles token lifecycle automatically:

- Tokens are validated before API requests
- Expired tokens are automatically refreshed
- If refresh fails, you'll be prompted to re-authenticate
- You can clear stored credentials by deleting the keychain entry

## Troubleshooting

### Authentication Issues

**Problem**: "SPOTIFY_CLIENT_ID environment variable not set"

- **Solution**: Ensure you've set the required environment variables (see Setup section)

**Problem**: "Token refresh failed, re-authentication required"

- **Solution**: The refresh token may have been revoked. Clear the keychain entry and re-authenticate:
  ```bash
  # Open Keychain Access app and delete the entry for "com.lyricsify.spotify"
  # Or use the app's re-authentication flow
  ```

**Problem**: Authentication callback not working

- **Solution**: Verify the redirect URI in your Spotify app settings matches `SPOTIFY_REDIRECT_URI`

### API Issues

**Problem**: "Failed to get currently playing track"

- **Solution**: Ensure Spotify is actively playing a track (not paused)
- **Note**: Podcasts are not supported for lyrics display

### Keychain Access

To manually manage stored credentials:

1. Open **Keychain Access** app
2. Search for `com.lyricsify.spotify`
3. Delete the entry to clear stored tokens

## License

MIT License - See LICENSE file for details

## Author

Aman Pratap Singh
