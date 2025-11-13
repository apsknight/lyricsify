# Project Structure

## Current State

This is an early-stage project. The repository currently contains only foundational files.

## Expected Rust Project Structure

When fully developed, the project will follow standard Rust conventions:

```
lyricsify/
├── src/              # Source code
│   ├── main.rs       # Application entry point
│   └── lib.rs        # Library code (if applicable)
├── tests/            # Integration tests
├── benches/          # Benchmarks
├── examples/         # Example usage code
├── target/           # Build output (gitignored)
├── Cargo.toml        # Project manifest and dependencies
├── Cargo.lock        # Dependency lock file
└── README.md         # Project documentation
```

## Conventions

### Source Organization

- Keep `main.rs` minimal - use it as an entry point
- Organize functionality into modules within `src/`
- Use `mod.rs` or module files for logical grouping
- Separate concerns: UI, Spotify API integration, lyrics fetching

### File Naming

- Use snake_case for file names (e.g., `spotify_client.rs`)
- Match module names to file names
- Keep related functionality together

### Ignored Directories

- `target/` - All build artifacts
- `mutants.out*/` - Mutation testing data
- `debug/` - Debug builds
