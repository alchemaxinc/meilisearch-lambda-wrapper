#![cfg(feature = "integration")]
mod common;

// The Proxy Forwarding integration test verifies that we've correctly implemented
// simple forwarding of the /keys and /index endpoint. If they work, we assume it all works.
// The more complex POST endpoint will then be handled separately as a part of the wrapping
// mechanism tests.
mod proxy_forwarding {
    use super::common;

    #[test]
    fn get_keys() {
        let ctx = common::TestContext::new();

        let response = ctx
            .get("/keys")
            .send()
            .expect("Failed to send get keys request");

        assert_eq!(
            response.status(),
            200,
            "Get keys failed with status {}",
            response.status()
        );

        let data: common::KeyListResponse =
            response.json().expect("Failed to parse keys response JSON");

        assert!(
            !data.results.is_empty(),
            "Expected at least one key in results"
        );
    }

    #[test]
    fn get_indexes_empty() {
        let ctx = common::TestContext::new();

        let response = ctx
            .get("/indexes")
            .send()
            .expect("Failed to send get indexes request");

        assert_eq!(
            response.status(),
            200,
            "Get indexes failed with status {}",
            response.status()
        );

        let data: common::IndexListResponse = response
            .json()
            .expect("Failed to parse indexes response JSON");

        assert_eq!(
            data.results.len(),
            0,
            "Expected no indexes on clean instance"
        );
        assert_eq!(data.offset, 0);
        assert_eq!(data.limit, 20);
        assert_eq!(data.total, 0);
    }
}

// Polling Wrapper tests if the POST endpoint is correctly wrapped with a POST/GET polling mechanism.
mod polling_wrapper {
    use std::{thread, time};

    use super::common;

    #[test]
    fn seed_and_verify_documents() {
        let ctx = common::TestContext::new();
        let csv_data = include_bytes!("fixtures/movies.csv");

        let response = ctx
            .post("/indexes/movies/documents?primaryKey=id")
            .header("Content-Type", "text/csv")
            .body(csv_data.as_slice())
            .send()
            .expect("Failed to send seed request");

        assert_eq!(
            response.status(),
            200,
            "Seed request failed with status {}",
            response.status()
        );

        // Poll until the indexing task completes
        let poll_interval = time::Duration::from_millis(common::POLL_INTERVAL_MS);

        let task = 'poll: {
            for attempt in 1..=common::MAX_POLL_ATTEMPTS {
                let response = ctx
                    .get("/tasks/0")
                    .send()
                    .expect("Failed to send task poll request");

                if response.status() != 200 {
                    eprintln!(
                        "Attempt {attempt}/{}: Task endpoint returned status {}, waiting...",
                        common::MAX_POLL_ATTEMPTS,
                        response.status()
                    );
                    thread::sleep(poll_interval);
                    continue;
                }

                let task: common::TaskResponse =
                    response.json().expect("Failed to parse task response JSON");

                if task.status == "succeeded" || task.status == "failed" {
                    break 'poll task;
                }

                eprintln!(
                    "Attempt {attempt}/{}: Task status is '{}', waiting...",
                    common::MAX_POLL_ATTEMPTS,
                    task.status
                );
                thread::sleep(poll_interval);
            }

            panic!(
                "Task did not complete within {} attempts",
                common::MAX_POLL_ATTEMPTS
            );
        };

        assert_eq!(task.details.received_documents, 31944);
        assert_eq!(task.details.indexed_documents, 31944);

        // Verify the index was created
        let response = ctx
            .get("/indexes")
            .send()
            .expect("Failed to send get indexes request");

        assert_eq!(response.status(), 200);

        let data: common::IndexListResponse = response
            .json()
            .expect("Failed to parse indexes response JSON");

        assert_eq!(data.total, 1);
        assert_eq!(data.results.len(), 1);
        assert_eq!(data.results[0].uid, "movies");
        assert_eq!(data.results[0].primary_key, "id");

        // Verify task list
        let response = ctx
            .get("/tasks")
            .send()
            .expect("Failed to send get tasks request");

        assert_eq!(response.status(), 200);

        let data: common::TaskListResponse = response
            .json()
            .expect("Failed to parse tasks response JSON");

        assert!(!data.results.is_empty(), "Expected at least one task");

        let task = &data.results[0];
        assert_eq!(task.index_uid, "movies");
        assert_eq!(task.status, "succeeded");
        assert_eq!(task.task_type, "documentAdditionOrUpdate");
        assert!(task.canceled_by.is_null(), "Expected canceledBy to be null");
        assert!(task.error.is_null(), "Expected error to be null");
        assert_eq!(task.details.received_documents, 31944);
        assert_eq!(task.details.indexed_documents, 31944);
    }
}
