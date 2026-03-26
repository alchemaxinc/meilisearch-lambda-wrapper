# Build stage: compile the Rust wrapper binary
FROM rust:1.86-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /build
COPY wrapper/ ./wrapper/

RUN cargo build \
    --manifest-path wrapper/Cargo.toml \
    --release

# Runtime stage: Meilisearch + compiled wrapper binary
FROM getmeili/meilisearch:v1.39.0

WORKDIR /app

COPY --from=builder /build/wrapper/target/release/wrapper ./wrapper
COPY --from=public.ecr.aws/awsguru/aws-lambda-adapter:0.9.1 /lambda-adapter /opt/extensions/lambda-adapter

ENTRYPOINT ["/app/wrapper"]
