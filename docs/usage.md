# Usage

Pre-built binaries are available on the
[Releases](https://github.com/alchemaxinc/meilisearch-lambda-wrapper/releases) page
for `x86_64` and `aarch64`.

## Dockerfile

```dockerfile
FROM alpine:3.21 AS fetcher

ARG WRAPPER_VERSION=1.0.0
ARG TARGETARCH

RUN apk add --no-cache curl && \
    case "${TARGETARCH}" in \
      amd64) RUST_TARGET="x86_64-unknown-linux-musl" ;; \
      arm64) RUST_TARGET="aarch64-unknown-linux-musl" ;; \
    esac && \
    curl -fsSL -o /wrapper \
      "https://github.com/alchemaxinc/meilisearch-lambda-wrapper/releases/download/v${WRAPPER_VERSION}/wrapper-${RUST_TARGET}" && \
    chmod +x /wrapper

FROM getmeili/meilisearch:v1.39.0
WORKDIR /app
COPY --from=fetcher /wrapper ./wrapper
COPY --from=public.ecr.aws/awsguru/aws-lambda-adapter:0.9.1 /lambda-adapter /opt/extensions/lambda-adapter
ENTRYPOINT ["/app/wrapper"]
```

Pin the version with a build arg:

```sh
docker build --build-arg WRAPPER_VERSION=1.2.3 .
```

## Verifying checksums

Each release includes `.sha256` files per binary:

```sh
curl -fsSL -O https://github.com/alchemaxinc/meilisearch-lambda-wrapper/releases/download/v1.2.3/wrapper-x86_64-unknown-linux-musl{,.sha256}
sha256sum -c wrapper-x86_64-unknown-linux-musl.sha256
```
