FROM getmeili/meilisearch:v1.35.0

RUN apk add --no-cache python3 py-pip

WORKDIR /app

COPY pyproject.toml ./pyproject.toml
COPY README.md ./README.md
COPY wrapper/ ./wrapper/

RUN python -m venv .venv && \
    source .venv/bin/activate && \
    pip3 install -e .

COPY --from=public.ecr.aws/awsguru/aws-lambda-adapter:0.9.1 /lambda-adapter /opt/extensions/lambda-adapter

ENTRYPOINT ["/app/.venv/bin/python", "-m", "wrapper.app"]
