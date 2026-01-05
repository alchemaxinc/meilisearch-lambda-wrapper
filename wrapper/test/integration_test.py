import logging
import os
import time
import unittest
from unittest import mock

import requests

logging.basicConfig(level=logging.INFO, format="%(asctime)s - %(message)s")


class MeiliSearchIntegrationTests(unittest.TestCase):

    PORT = os.environ.get("MEILI_PORT")
    HOST = os.environ.get("MEILI_HOST")
    MASTER_KEY = os.environ.get("MEILI_MASTER_KEY")

    if not PORT:
        raise EnvironmentError("MEILI_PORT environment variable is not set")

    if not HOST:
        raise EnvironmentError("MEILI_HOST environment variable is not set")

    if not MASTER_KEY:
        raise EnvironmentError("MEILI_MASTER_KEY environment variable is not set")

    BASE_URL = f"http://{HOST}:{PORT}"
    FIXTURES_DIR = os.path.join(os.path.dirname(__file__), "fixtures")

    def setUp(self):
        self.headers = {
            "Authorization": f"Bearer {self.MASTER_KEY}",
        }

    def _step_1_test_seed_indexes(self):
        csv_file_path = os.path.join(self.FIXTURES_DIR, "movies.csv")
        with open(csv_file_path, "rb") as f:
            csv_data = f.read()

        url = f"{self.BASE_URL}/indexes/movies/documents?primaryKey=id"
        headers = self.headers.copy()
        headers["Content-Type"] = "text/csv"

        response = requests.post(url, headers=headers, data=csv_data)

        self.assertEqual(
            response.status_code,
            200,
            f"Expected status 200, got {response.status_code}",
        )

    def _step_2_test_poll_task_by_id(self):
        # Poll the task endpoint until completion
        task_url = f"{self.BASE_URL}/tasks/0"
        max_attempts = 30
        for attempt in range(max_attempts):
            task_response = requests.get(task_url, headers=self.headers)
            if task_response.status_code != 200:
                logging.info(
                    f"Attempt {attempt + 1}/{max_attempts}: Task endpoint returned status {task_response.status_code}, waiting..."
                )
                time.sleep(0.5)
                continue

            task_data = task_response.json()
            task_status = task_data.get("status")
            if task_status not in ["succeeded", "failed"]:
                logging.info(
                    f"Attempt {attempt + 1}/{max_attempts}: Task status is '{task_status}', waiting..."
                )
                time.sleep(0.5)
                continue

            expected_details = {
                "receivedDocuments": 100,
                "indexedDocuments": 100,
            }

            actual_details = {
                "receivedDocuments": task_data["details"]["receivedDocuments"],
                "indexedDocuments": task_data["details"]["indexedDocuments"],
            }
            self.assertEqual(expected_details, actual_details)
            break

    def _step_3_test_get_indexes(self):
        url = f"{self.BASE_URL}/indexes"

        response = requests.get(url, headers=self.headers)

        self.assertEqual(
            response.status_code,
            200,
            f"Expected status 200, got {response.status_code}",
        )

        response_data = response.json()

        expected_response = {
            "results": [
                {
                    "uid": "person",
                    "createdAt": mock.ANY,
                    "updatedAt": mock.ANY,
                    "primaryKey": "id",
                }
            ],
            "offset": 0,
            "limit": 20,
            "total": 1,
        }

        self.assertEqual(expected_response, response_data)

    def _step_4_test_get_all_tasks(self):
        """Query the /tasks endpoint without a task ID and verify the response"""
        url = f"{self.BASE_URL}/tasks"

        response = requests.get(url, headers=self.headers)

        self.assertEqual(
            response.status_code,
            200,
            f"Expected status 200, got {response.status_code}",
        )

        response_data = response.json()

        # Verify the first task matches the expected structure
        expected_task = {
            "uid": mock.ANY,
            "batchUid": mock.ANY,
            "indexUid": "person",
            "status": "succeeded",
            "type": "documentAdditionOrUpdate",
            "canceledBy": None,
            "details": {
                "receivedDocuments": 100,
                "indexedDocuments": 100,
            },
            "error": None,
            "duration": mock.ANY,
            "enqueuedAt": mock.ANY,
            "startedAt": mock.ANY,
            "finishedAt": mock.ANY,
        }

        self.assertIn("results", response_data)
        self.assertGreater(len(response_data["results"]), 0)
        self.assertEqual(expected_task, response_data["results"][0])

    def _steps(self):
        for name in dir(self):  # dir() result is implicitly sorted
            if not name.startswith("_step"):
                continue

            yield name, getattr(self, name)

    def test_steps(self):
        for name, step in self._steps():
            try:
                step()
            except Exception as e:
                self.fail("{} failed ({}: {})".format(step, type(e), e))
