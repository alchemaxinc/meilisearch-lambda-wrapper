FROM rust:1.86-alpine

RUN apk add --no-cache musl-dev

WORKDIR /build
COPY wrapper/ ./wrapper/

RUN cargo test \
    --manifest-path wrapper/Cargo.toml \
    --features integration \
    --test integration_test \
    --no-run

CMD ["cargo", "test", "--manifest-path", "wrapper/Cargo.toml", "--features", "integration", "--test", "integration_test", "--", "--test-threads=1"]
