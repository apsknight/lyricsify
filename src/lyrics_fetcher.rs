use crate::error::LyricsifyError;
use reqwest::Client;
use serde::Deserialize;
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

/// Represents lyrics data with optional content
#[derive(Debug, Clone)]
pub struct Lyrics {
    pub text: Option<String>,
    pub source: String,
}

/// Cached lyrics entry with timestamp for LRU eviction
#[derive(Debug, Clone)]
struct CachedLyrics {
    lyrics: Option<String>,
    timestamp: Instant,
}

/// LRU cache for lyrics
struct LyricsCache {
    entries: HashMap<String, CachedLyrics>,
    access_order: VecDeque<String>,
    max_size: usize,
}

impl LyricsCache {
    fn new(max_size: usize) -> Self {
        Self {
            entries: HashMap::new(),
            access_order: VecDeque::new(),
            max_size,
        }
    }

    fn get(&mut self, track_id: &str) -> Option<&CachedLyrics> {
        if self.entries.contains_key(track_id) {
            // Update access order - move to back (most recently used)
            self.access_order.retain(|id| id != track_id);
            self.access_order.push_back(track_id.to_string());
            self.entries.get(track_id)
        } else {
            None
        }
    }

    fn insert(&mut self, track_id: String, lyrics: Option<String>) {
        // If cache is full, evict least recently used entry
        if self.entries.len() >= self.max_size && !self.entries.contains_key(&track_id) {
            if let Some(lru_key) = self.access_order.pop_front() {
                self.entries.remove(&lru_key);
                log::debug!("Evicted LRU cache entry: {}", lru_key);
            }
        }

        // Insert or update entry
        let cached = CachedLyrics {
            lyrics,
            timestamp: Instant::now(),
        };
        
        if self.entries.contains_key(&track_id) {
            // Update existing entry
            self.entries.insert(track_id.clone(), cached);
            // Update access order
            self.access_order.retain(|id| id != &track_id);
            self.access_order.push_back(track_id);
        } else {
            // Insert new entry
            self.entries.insert(track_id.clone(), cached);
            self.access_order.push_back(track_id);
        }
    }
}

/// Response structure from Lyrics.ovh API
#[derive(Debug, Deserialize)]
struct LyricsOvhResponse {
    lyrics: String,
}

/// Main lyrics fetcher with HTTP client and caching
pub struct LyricsFetcher {
    http_client: Client,
    cache: LyricsCache,
}

impl LyricsFetcher {
    /// Create a new LyricsFetcher with configured HTTP client
    pub fn new() -> Result<Self, LyricsifyError> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;

        Ok(Self {
            http_client,
            cache: LyricsCache::new(100),
        })
    }

    /// Fetch lyrics for a track, using cache if available
    pub async fn fetch_lyrics(
        &mut self,
        track_id: &str,
        artist: &str,
        title: &str,
    ) -> Result<Option<String>, LyricsifyError> {
        // Check cache first
        if let Some(cached) = self.cache.get(track_id) {
            log::debug!("Cache hit for track: {}", track_id);
            return Ok(cached.lyrics.clone());
        }

        log::info!("Fetching lyrics for: {} - {}", artist, title);

        // Fetch from API
        match self.query_lyrics_ovh(artist, title).await {
            Ok(lyrics) => {
                log::info!("Successfully fetched lyrics for: {} - {}", artist, title);
                self.cache.insert(track_id.to_string(), Some(lyrics.clone()));
                Ok(Some(lyrics))
            }
            Err(e) => {
                log::warn!("Failed to fetch lyrics for {} - {}: {}", artist, title, e);
                // Cache negative result to avoid repeated failed lookups
                self.cache.insert(track_id.to_string(), None);
                Ok(None)
            }
        }
    }

    /// Query Lyrics.ovh API for lyrics
    async fn query_lyrics_ovh(&self, artist: &str, title: &str) -> Result<String, LyricsifyError> {
        // URL-encode artist and title for path parameters
        let encoded_artist = urlencoding::encode(artist);
        let encoded_title = urlencoding::encode(title);

        let url = format!(
            "https://api.lyrics.ovh/v1/{}/{}",
            encoded_artist, encoded_title
        );

        log::debug!("Querying Lyrics.ovh: {}", url);

        let response = self.http_client.get(&url).send().await?;

        if response.status().is_success() {
            let lyrics_response: LyricsOvhResponse = response.json().await?;
            Ok(lyrics_response.lyrics)
        } else if response.status().as_u16() == 404 {
            Err(LyricsifyError::LyricsFetchError(
                "Lyrics not found".to_string(),
            ))
        } else {
            Err(LyricsifyError::LyricsFetchError(format!(
                "API returned status: {}",
                response.status()
            )))
        }
    }
}
