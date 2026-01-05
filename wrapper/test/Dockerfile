FROM python:3.13-alpine

WORKDIR /app

# Copy the requirements and fixtures
COPY requirements.txt ./requirements.txt
COPY test/fixtures ./fixtures
COPY test/integration_test.py ./integration_test.py

# Install dependencies
RUN pip install --no-cache-dir -r requirements.txt

# Run the integration tests
CMD ["python", "-m", "unittest", "integration_test.MeiliSearchIntegrationTests.test_steps", "-v"]
