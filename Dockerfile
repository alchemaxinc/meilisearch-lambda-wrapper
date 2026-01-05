FROM getmeili/meilisearch:v1.29

RUN apk add --no-cache python3 py-pip

WORKDIR /app

COPY requirements.txt ./requirements.txt
RUN python -m venv .venv && \
    source .venv/bin/activate && \
    pip3 install -r ./requirements.txt

COPY wrapper/ ./wrapper/
RUN chmod +x ./wrapper/app.py

COPY --from=public.ecr.aws/awsguru/aws-lambda-adapter:0.9.1 /lambda-adapter /opt/extensions/lambda-adapter

ENTRYPOINT ["/app/.venv/bin/python", "/app/wrapper/app.py"]
