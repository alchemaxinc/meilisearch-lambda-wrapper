# Build stage: compile the Rust wrapper binary
# RUST_VERSION is sourced from rust-toolchain.toml so the cargo updater
# (which bumps the channel field) is the single source of truth.
ARG RUST_VERSION=1.95.0
FROM rust:${RUST_VERSION}-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /build

# Cache dependency layer: only re-built when Cargo.toml or Cargo.lock change
COPY wrapper/Cargo.toml wrapper/Cargo.lock ./wrapper/
RUN mkdir -p wrapper/src && echo "fn main() {}" > wrapper/src/main.rs
RUN cargo build \
    --manifest-path wrapper/Cargo.toml \
    --release

# Build actual source
COPY wrapper/src/ ./wrapper/src/
RUN touch wrapper/src/main.rs && cargo build \
    --manifest-path wrapper/Cargo.toml \
    --release

# Runtime stage: Meilisearch + compiled wrapper binary
FROM getmeili/meilisearch:v1.52

WORKDIR /app

COPY --from=builder /build/wrapper/target/release/wrapper ./wrapper
COPY --from=public.ecr.aws/awsguru/aws-lambda-adapter:1.0.0 /lambda-adapter /opt/extensions/lambda-adapter

ENTRYPOINT ["/app/wrapper"]
