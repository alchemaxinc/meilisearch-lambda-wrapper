FROM rust:1.86-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /build
COPY wrapper/ ./wrapper/

RUN cargo test \
    --manifest-path wrapper/Cargo.toml \
    --test integration_test \
    --no-run

RUN cp wrapper/target/debug/deps/integration_test-* /usr/local/bin/integration_test && \
    rm -f /usr/local/bin/*.d

FROM alpine:3.22

COPY --from=builder /usr/local/bin/integration_test /usr/local/bin/integration_test

CMD ["integration_test", "--test-threads=1"]
