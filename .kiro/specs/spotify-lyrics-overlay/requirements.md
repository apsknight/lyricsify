# Requirements Document

## Introduction

Lyricsify is a macOS application that displays real-time lyrics for currently playing Spotify tracks as a translucent overlay. The application integrates with Spotify's API to detect the current track and fetches corresponding lyrics, presenting them in a non-intrusive, always-on-top window. Users can toggle the overlay visibility through a menu bar icon, making it ideal for multitasking scenarios where users want to view lyrics while working.

## Glossary

- **Lyrics Overlay**: A translucent, always-on-top window that displays song lyrics
- **Menu Bar Icon**: A clickable icon in the macOS menu bar that provides access to application controls
- **Spotify Client**: The component responsible for communicating with Spotify's API using the rspotify Rust library
- **Lyrics Fetcher**: The component responsible for retrieving lyrics from external sources
- **Current Track**: The song currently playing in the user's Spotify application
- **Authentication Token**: OAuth credentials used to authorize API requests to Spotify
- **rspotify**: A Rust library that provides a wrapper around the Spotify Web API

## Requirements

### Requirement 1

**User Story:** As a Spotify user, I want to authenticate with my Spotify account, so that the application can access my currently playing track information

#### Acceptance Criteria

1. WHEN the user launches the application for the first time, THE Lyrics Overlay SHALL initiate the Spotify OAuth authentication flow
2. WHEN the user completes authentication, THE Lyrics Overlay SHALL store the Authentication Token securely in the macOS keychain
3. IF the Authentication Token expires, THEN THE Lyrics Overlay SHALL prompt the user to re-authenticate
4. WHEN authentication succeeds, THE Lyrics Overlay SHALL display a success notification to the user

### Requirement 2

**User Story:** As a user, I want the application to detect what song is currently playing on Spotify, so that relevant lyrics can be displayed

#### Acceptance Criteria

1. WHILE the application is running, THE Spotify Client SHALL use rspotify to poll the Spotify API every 5 seconds to retrieve Current Track information
2. WHEN the Current Track changes, THE Spotify Client SHALL notify the Lyrics Fetcher with the new track details
3. THE Spotify Client SHALL extract the track name and artist name from the rspotify API response
4. IF the Spotify API returns an error, THEN THE Spotify Client SHALL retry the request up to 3 times with exponential backoff

### Requirement 3

**User Story:** As a user, I want the application to automatically fetch lyrics for the currently playing song, so that I can read along without manual intervention

#### Acceptance Criteria

1. WHEN the Spotify Client detects a new Current Track, THE Lyrics Fetcher SHALL request lyrics using the track name and artist name
2. THE Lyrics Fetcher SHALL retrieve lyrics from at least one external lyrics API service
3. WHEN lyrics are successfully retrieved, THE Lyrics Fetcher SHALL parse and format the lyrics for display
4. IF lyrics are not available for the Current Track, THEN THE Lyrics Overlay SHALL display a message indicating lyrics are unavailable
5. THE Lyrics Fetcher SHALL cache retrieved lyrics to minimize API requests for repeated tracks

### Requirement 4

**User Story:** As a user, I want to see lyrics displayed as a translucent overlay on my screen, so that I can view them while working on other tasks

#### Acceptance Criteria

1. WHEN lyrics are available, THE Lyrics Overlay SHALL display them in a translucent window with 80% opacity
2. THE Lyrics Overlay SHALL position the window at a default location on the screen that minimizes obstruction
3. THE Lyrics Overlay SHALL remain always-on-top of other application windows
4. THE Lyrics Overlay SHALL allow the user to drag the window to reposition it
5. THE Lyrics Overlay SHALL remember the user's preferred window position between application sessions
6. THE Lyrics Overlay SHALL use a readable font with sufficient contrast against the translucent background

### Requirement 5

**User Story:** As a user, I want to toggle the lyrics overlay visibility through a menu bar icon, so that I can quickly show or hide lyrics without closing the application

#### Acceptance Criteria

1. WHEN the application launches, THE Lyrics Overlay SHALL create a Menu Bar Icon in the macOS menu bar
2. WHEN the user clicks the Menu Bar Icon, THE Lyrics Overlay SHALL display a dropdown menu with toggle options
3. WHEN the user selects "Show Lyrics" from the menu, THE Lyrics Overlay SHALL make the overlay window visible
4. WHEN the user selects "Hide Lyrics" from the menu, THE Lyrics Overlay SHALL hide the overlay window without terminating the application
5. THE Menu Bar Icon SHALL display different visual states to indicate whether the overlay is currently visible or hidden

### Requirement 6

**User Story:** As a user, I want the application to run efficiently in the background, so that it doesn't impact my system performance while multitasking

#### Acceptance Criteria

1. THE Lyrics Overlay SHALL consume less than 50 MB of memory during normal operation
2. THE Spotify Client SHALL limit API polling to prevent excessive network requests
3. THE Lyrics Overlay SHALL use native macOS APIs to minimize CPU usage
4. WHEN the overlay is hidden, THE Lyrics Overlay SHALL reduce background activity to minimal levels

### Requirement 7

**User Story:** As a user, I want to quit the application from the menu bar, so that I can cleanly exit when I no longer need lyrics

#### Acceptance Criteria

1. WHEN the user clicks the Menu Bar Icon, THE Lyrics Overlay SHALL include a "Quit" option in the dropdown menu
2. WHEN the user selects "Quit", THE Lyrics Overlay SHALL terminate all background processes
3. WHEN the user selects "Quit", THE Lyrics Overlay SHALL save the current window position and preferences
4. THE Lyrics Overlay SHALL perform cleanup operations before terminating
