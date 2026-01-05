FROM python:3.13-alpine

WORKDIR /app

COPY requirements.txt ./requirements.txt
COPY wrapper/test/fixtures ./fixtures
COPY wrapper/test/integration_test.py ./integration_test.py

RUN pip install --no-cache-dir -r requirements.txt

CMD ["python", "-m", "unittest", "integration_test.MeiliSearchIntegrationTests.test_steps", "-v"]
