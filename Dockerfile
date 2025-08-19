FROM rust:1.88.0-slim AS chef

# Fix potential vulnerabilities and install build dependencies
RUN apt-get update && apt-get upgrade -y && apt-get install -y --no-install-recommends ca-certificates pkg-config libssl-dev && apt-get clean && rm -rf /var/lib/apt/lists/*

RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY ./src/ /app/src/
COPY ./migration/ /app/migration/
COPY Cargo.lock Cargo.toml /app/
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

RUN cargo chef cook --release --recipe-path recipe.json

# Build application
COPY ./src /app/src
COPY ./migration/ /app/migration/
COPY Cargo.lock Cargo.toml /app/

RUN cargo build --release --bin spice-api

FROM debian:bookworm-slim AS runtime

# Fix potential vulnerabilities
RUN apt-get update && apt-get upgrade -y && apt-get install -y --no-install-recommends openssl ca-certificates && apt-get clean && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/spice-api /usr/local/bin
ENTRYPOINT ["/usr/local/bin/spice-api"]
