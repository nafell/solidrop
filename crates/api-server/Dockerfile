# -- Planner stage: compute dependency recipe --------------------------------
FROM lukemathwalker/cargo-chef:latest-rust-1.93-slim-bookworm AS chef
WORKDIR /app

FROM chef AS planner
COPY Cargo.toml ./
COPY crates/crypto/ crates/crypto/
COPY crates/api-server/ crates/api-server/
# CLI stub: workspace member must exist for `cargo chef prepare`
COPY crates/cli/Cargo.toml crates/cli/Cargo.toml
RUN mkdir -p crates/cli/src && echo "fn main() {}" > crates/cli/src/main.rs
RUN cargo chef prepare --recipe-path recipe.json

# -- Builder stage: cook deps (cached) then build binary ---------------------
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Cook all workspace dependencies (this layer is cached unless Cargo.* changes)
RUN cargo chef cook --release --recipe-path recipe.json

COPY Cargo.toml ./
COPY crates/crypto/ crates/crypto/
COPY crates/api-server/ crates/api-server/
COPY crates/cli/Cargo.toml crates/cli/Cargo.toml
RUN mkdir -p crates/cli/src && echo "fn main() {}" > crates/cli/src/main.rs
RUN cargo build --release --package solidrop-api-server

# -- Runtime stage -----------------------------------------------------------
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/solidrop-api-server /usr/local/bin/solidrop-api-server

EXPOSE 3000
CMD ["solidrop-api-server"]
