# Consumer Usage

To use the wrapper in your own project, create a Dockerfile that installs it
directly from this repository using `cargo install`:

```dockerfile
# Build stage: compile the wrapper binary from the repository
FROM rust:1.94-alpine AS builder
RUN apk add --no-cache musl-dev
RUN cargo install \
    --git https://github.com/alchemaxinc/meilisearch-lambda-wrapper.git \
    --path wrapper

# Runtime stage: Meilisearch + compiled wrapper binary
FROM getmeili/meilisearch:v1.39.0
WORKDIR /app
COPY --from=builder /usr/local/cargo/bin/wrapper ./wrapper
COPY --from=public.ecr.aws/awsguru/aws-lambda-adapter:0.9.1 /lambda-adapter /opt/extensions/lambda-adapter
ENTRYPOINT ["/app/wrapper"]
```

## Version pinning

To pin to a specific release, use the `--tag` flag:

```dockerfile
RUN cargo install \
    --git https://github.com/alchemaxinc/meilisearch-lambda-wrapper.git \
    --tag v1.0.0 \
    --path wrapper
```
