# Technology Stack

## Language & Build System

- **Language**: Rust
- **Build System**: Cargo (Rust's package manager and build tool)

## Development Tools

- **Testing**: cargo mutants (mutation testing)
- **Formatting**: rustfmt (Rust code formatter)
- **IDE Support**: RustRover, VS Code with rust-analyzer

## Common Commands

### Building

```bash
cargo build          # Debug build
cargo build --release # Optimized release build
```

### Running

```bash
cargo run            # Build and run debug version
cargo run --release  # Build and run release version
```

### Testing

```bash
cargo test           # Run all tests
cargo test --release # Run tests in release mode
cargo mutants        # Run mutation testing
```

### Code Quality

```bash
cargo fmt            # Format code
cargo clippy         # Run linter
cargo check          # Fast compile check without producing binary
```

### Cleaning

```bash
cargo clean          # Remove build artifacts
```

## Build Artifacts

- `target/debug/` - Debug builds
- `target/release/` - Release builds
- `mutants.out*/` - Mutation testing results
- `**/*.rs.bk` - rustfmt backup files
