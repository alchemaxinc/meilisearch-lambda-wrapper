FROM getmeili/meilisearch:v1.29

# Install Python 3 using apk (Alpine package manager)
RUN apk add --no-cache python3 py-pip

# Set working directory
WORKDIR /app

# Copy requirements and install
COPY requirements.txt ./requirements.txt
RUN python -m venv .venv && \
    source .venv/bin/activate && \
    pip3 install -r ./requirements.txt

# Copy wrapper package and main entry point
COPY wrapper/ ./wrapper/
RUN chmod +x ./wrapper/app.py

# Copy Lambda adapter for Lambda Web Adapter
COPY --from=public.ecr.aws/awsguru/aws-lambda-adapter:0.9.1 /lambda-adapter /opt/extensions/lambda-adapter

# Then start the Python wrapper
ENTRYPOINT ["/app/.venv/bin/python", "/app/wrapper/app.py"]
