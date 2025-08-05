# SPICE API

**Submicron Particle Ice Nucleation (SPICE) API** - A Rust web application for managing ice nucleation particle experiment data in the EERL (Environmental and Energy Research Laboratory). Built with Axum and SeaORM, featuring complex Excel data processing for 96-well plate scientific experiments.

## Development

### Running Tests
```bash
# Run all tests
cargo test

# Run specific test module
cargo test treatments::tests -- --nocapture

# Run tests for a specific route
cargo test routes::samples -- --nocapture
```

### Code Coverage
```bash
# Install coverage tools (one-time setup)
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov

# Generate coverage report
cargo llvm-cov --workspace --html --ignore-filename-regex="migration"

# View coverage report
open target/llvm-cov/html/index.html
```

### Key Commands
```bash
# Development with live reloading
bacon run

# Code quality checks  
bacon clippy-all

# Database migrations (from migration/ directory)
cargo run
```
