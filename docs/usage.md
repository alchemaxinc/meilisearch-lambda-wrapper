# Consumer Usage

To use the wrapper in your own project, create a Dockerfile that installs it
directly from this repository using `git clone` + `cargo install`:

```dockerfile
# Build stage: compile the wrapper binary from the repository
FROM rust:1.94-alpine AS builder
RUN apk add --no-cache musl-dev git
RUN git clone --depth 1 \
    https://github.com/alchemaxinc/meilisearch-lambda-wrapper.git /tmp/repo && \
    cargo install --path /tmp/repo/wrapper

# Runtime stage: Meilisearch + compiled wrapper binary
FROM getmeili/meilisearch:v1.39.0
WORKDIR /app
COPY --from=builder /usr/local/cargo/bin/meilisearch-lambda-wrapper ./wrapper
COPY --from=public.ecr.aws/awsguru/aws-lambda-adapter:0.9.1 /lambda-adapter /opt/extensions/lambda-adapter
ENTRYPOINT ["/app/wrapper"]
```

## Version pinning

To pin to a specific branch or tag, use the `--branch` flag:

```dockerfile
RUN git clone --branch v1.0.0 --depth 1 \
    https://github.com/alchemaxinc/meilisearch-lambda-wrapper.git /tmp/repo && \
    cargo install --path /tmp/repo/wrapper
```
