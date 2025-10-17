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

## Database Seeding

Populate the database with realistic test data using the built-in seeder:

```bash
# Run the database seeder (requires JWT token)
cargo run --bin seed_database -- --url http://localhost:3000 --token YOUR_JWT_TOKEN
```

**What it creates:**
- **4 Research Projects** (Arctic, Agricultural, Marine, Climate)
- **12 Sampling Locations** (3 per project with realistic GPS coordinates)
- **1,200 Environmental Samples** (100 per location with geographic spread within 1km)
- **~2,400 Laboratory Treatments** (based on sample type)
- **1 Realistic Tray Configuration** (INP freezing assay with P1/P2 trays, 4 probes each, precise positioning)
- **3 Experiments** with realistic parameters (linked to tray configuration)
- **Excel Processing** of merged.xlsx file

**Features:**
- Realistic GPS coordinates with geographic distribution within 1km radius
- Diverse sample types including blanks and quality controls
- Varied collection dates spanning several months
- Production-ready INP freezing assay tray configuration with precise probe positioning
- Proper tray configuration linking for experiments
- Beautiful progress indicators with error handling
