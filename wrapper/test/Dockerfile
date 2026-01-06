FROM python:3.13-alpine

WORKDIR /app

COPY pyproject.toml ./pyproject.toml
COPY README.md ./README.md
COPY wrapper/__init__.py ./wrapper/__init__.py
COPY wrapper/test/fixtures ./fixtures
COPY wrapper/test/integration_test.py ./integration_test.py

RUN pip install --no-cache-dir .

CMD ["python", "-m", "unittest", "integration_test.MeiliSearchIntegrationTests.test_steps", "-v"]
