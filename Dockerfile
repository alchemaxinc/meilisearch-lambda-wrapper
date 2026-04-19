# Build stage: compile the Rust wrapper binary
FROM rust:1.94-alpine AS builder

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
FROM getmeili/meilisearch:v1.42.1

WORKDIR /app

COPY --from=builder /build/wrapper/target/release/wrapper ./wrapper
COPY --from=public.ecr.aws/awsguru/aws-lambda-adapter:1.0.0 /lambda-adapter /opt/extensions/lambda-adapter

ENTRYPOINT ["/app/wrapper"]
