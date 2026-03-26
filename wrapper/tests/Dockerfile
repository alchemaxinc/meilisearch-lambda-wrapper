FROM rust:1.94-alpine

RUN apk add --no-cache musl-dev

WORKDIR /build

# Cache dependency layer: only re-built when Cargo.toml or Cargo.lock change
COPY wrapper/Cargo.toml wrapper/Cargo.lock ./wrapper/
RUN mkdir -p wrapper/src && echo "fn main() {}" > wrapper/src/main.rs
RUN cargo build \
    --manifest-path wrapper/Cargo.toml \
    --features integration

# Build actual test binary
COPY wrapper/src/ ./wrapper/src/
COPY wrapper/tests/ ./wrapper/tests/
RUN touch wrapper/src/main.rs && cargo test \
    --manifest-path wrapper/Cargo.toml \
    --features integration \
    --test integration_test \
    --no-run

CMD ["cargo", "test", "--manifest-path", "wrapper/Cargo.toml", "--features", "integration", "--test", "integration_test", "--", "--test-threads=1"]
