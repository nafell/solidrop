# Build stage
FROM rust:1.93-slim AS builder

WORKDIR /app

# Copy workspace files
COPY Cargo.toml ./
COPY crates/crypto/ crates/crypto/
COPY crates/api-server/ crates/api-server/
# CLI is needed for workspace resolution but not built
COPY crates/cli/Cargo.toml crates/cli/Cargo.toml
RUN mkdir -p crates/cli/src && echo "fn main() {}" > crates/cli/src/main.rs

RUN cargo build --release --package solidrop-api-server

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/solidrop-api-server /usr/local/bin/solidrop-api-server

EXPOSE 3000

CMD ["solidrop-api-server"]
