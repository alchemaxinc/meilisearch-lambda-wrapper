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

        let response = ctx.get("/keys").send().expect("Failed to send get keys request");

        assert_eq!(
            response.status(),
            200,
            "Get keys failed with status {}",
            response.status()
        );

        let data: common::KeyListResponse = response.json().expect("Failed to parse keys response JSON");

        assert!(!data.results.is_empty(), "Expected at least one key in results");
    }

    // Verify the /indexes endpoint returns a valid response with the expected shape.
    // We cannot assert specific values (e.g. empty results) because test execution
    // order is not guaranteed — another test may have already created indexes.
    #[test]
    fn get_indexes() {
        let ctx = common::TestContext::new();

        let response = ctx.get("/indexes").send().expect("Failed to send get indexes request");

        assert_eq!(
            response.status(),
            200,
            "Get indexes failed with status {}",
            response.status()
        );

        let data: common::IndexListResponse = response.json().expect("Failed to parse indexes response JSON");

        assert_eq!(data.offset, 0, "Expected offset to be 0");
        assert!(data.limit > 0, "Expected limit to be greater than 0");
    }
}

// Polling Wrapper tests if the POST endpoint is correctly wrapped with a POST/GET polling mechanism.
// The proxy should handle polling internally and return the completed task response directly.
mod polling_wrapper {
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

        let task: common::TaskResponse = response.json().expect("Failed to parse task response JSON");

        // The proxy should have waited for the task to complete.
        // Should NOT return an enqueued response.
        assert_eq!(
            task.status, "succeeded",
            "Expected proxy to return completed task, got '{}'. If 'enqueued', the proxy is not polling.",
            task.status
        );
        assert_eq!(task.details.received_documents, 31944);
        assert_eq!(task.details.indexed_documents, 31944);

        // Verify the index was created. Do not assert the total index count:
        // other tests may run in the same Meilisearch instance and create
        // their own indexes.
        let response = ctx.get("/indexes").send().expect("Failed to send get indexes request");

        assert_eq!(response.status(), 200);

        let data: common::IndexListResponse = response.json().expect("Failed to parse indexes response JSON");

        assert!(data.total >= 1, "Expected at least one index to exist");
        let movies_index = data
            .results
            .iter()
            .find(|index| return index.uid == "movies")
            .expect("Expected a 'movies' index to exist");
        assert_eq!(movies_index.primary_key.as_deref(), Some("id"));

        // Verify task list
        let response = ctx.get("/tasks").send().expect("Failed to send get tasks request");

        assert_eq!(response.status(), 200);

        let data: common::TaskListResponse = response.json().expect("Failed to parse tasks response JSON");

        assert!(!data.results.is_empty(), "Expected at least one task");

        // Other tests may run against the same Meilisearch instance and add
        // their own tasks. Find this test's task instead of assuming index 0.
        let task = data
            .results
            .iter()
            .find(|t| return t.index_uid.as_deref() == Some("movies") && t.task_type == "documentAdditionOrUpdate")
            .expect("Expected a documentAdditionOrUpdate task for the 'movies' index");
        assert_eq!(task.status, "succeeded");
        assert!(task.canceled_by.is_null(), "Expected canceledBy to be null");
        assert!(task.error.is_null(), "Expected error to be null");
        assert_eq!(task.details["receivedDocuments"], 31944);
        assert_eq!(task.details["indexedDocuments"], 31944);
    }
}

// Every async Meilisearch operation must wait for the task to finish. This
// is not only true for `POST /indexes/{*rest}`. Meilisearch returns a
// summarized task with a `taskUid` for each of these operations. Task
// cancellation and deletion return status 200, not 202. So the proxy must
// check the response body for a `taskUid`, not the method or status code.
mod async_route_coverage {
    use super::common;

    #[test]
    fn patch_settings_is_synchronous() {
        let ctx = common::TestContext::new();

        let response = ctx
            .post("/indexes")
            .json(&serde_json::json!({"uid": "async_settings_test", "primaryKey": "id"}))
            .send()
            .expect("Failed to send create index request");
        assert_eq!(response.status(), 200);
        let task: common::SimpleTaskResponse = response.json().expect("Failed to parse task response JSON");
        assert_eq!(task.status, "succeeded");

        // Before this fix, the proxy did not intercept PATCH. It would
        // return the raw task with status 202, and would not wait for it.
        let response = ctx
            .patch("/indexes/async_settings_test/settings")
            .json(&serde_json::json!({"searchableAttributes": ["title"]}))
            .send()
            .expect("Failed to send patch settings request");

        assert_eq!(
            response.status(),
            200,
            "PATCH settings failed with status {}",
            response.status()
        );
        let task: common::SimpleTaskResponse = response.json().expect("Failed to parse task response JSON");
        assert_eq!(
            task.status, "succeeded",
            "Expected proxy to wait for the settings update task, got '{}'",
            task.status
        );
        assert_eq!(task.task_type, "settingsUpdate");
    }

    #[test]
    fn put_documents_and_delete_document_are_synchronous() {
        let ctx = common::TestContext::new();

        let response = ctx
            .post("/indexes")
            .json(&serde_json::json!({"uid": "async_docs_test", "primaryKey": "id"}))
            .send()
            .expect("Failed to send create index request");
        assert_eq!(response.status(), 200);

        // Before this fix, the proxy did not intercept PUT.
        let response = ctx
            .put("/indexes/async_docs_test/documents")
            .json(&serde_json::json!([{"id": 1, "title": "a"}]))
            .send()
            .expect("Failed to send put documents request");

        assert_eq!(
            response.status(),
            200,
            "PUT documents failed with status {}",
            response.status()
        );
        let task: common::SimpleTaskResponse = response.json().expect("Failed to parse task response JSON");
        assert_eq!(task.status, "succeeded");
        assert_eq!(task.task_type, "documentAdditionOrUpdate");

        // Before this fix, the proxy did not intercept DELETE.
        let response = ctx
            .delete("/indexes/async_docs_test/documents/1")
            .send()
            .expect("Failed to send delete document request");

        assert_eq!(
            response.status(),
            200,
            "DELETE document failed with status {}",
            response.status()
        );
        let task: common::SimpleTaskResponse = response.json().expect("Failed to parse task response JSON");
        assert_eq!(
            task.status, "succeeded",
            "Expected proxy to wait for the document deletion task, got '{}'",
            task.status
        );
        assert_eq!(task.task_type, "documentDeletion");
    }

    // Task cancellation and task deletion return status 200, not 202, but
    // they still enqueue a task. This test checks that detection does not
    // depend on status 202.
    #[test]
    fn delete_tasks_is_synchronous() {
        let ctx = common::TestContext::new();

        let response = ctx
            .delete("/tasks?statuses=succeeded,failed,canceled")
            .send()
            .expect("Failed to send delete tasks request");

        assert_eq!(
            response.status(),
            200,
            "DELETE tasks failed with status {}",
            response.status()
        );
        let task: common::SimpleTaskResponse = response.json().expect("Failed to parse task response JSON");
        assert_eq!(
            task.status, "succeeded",
            "Expected proxy to wait for the task deletion task, got '{}'",
            task.status
        );
        assert_eq!(task.task_type, "taskDeletion");
    }

    // A search is a sync operation, not a task. The proxy must pass it
    // through with no delay from the task-polling logic.
    #[test]
    fn search_is_not_intercepted() {
        let ctx = common::TestContext::new();

        let response = ctx
            .post("/indexes")
            .json(&serde_json::json!({"uid": "async_search_test", "primaryKey": "id"}))
            .send()
            .expect("Failed to send create index request");
        assert_eq!(response.status(), 200);

        let response = ctx
            .post("/indexes/async_search_test/search")
            .json(&serde_json::json!({"q": ""}))
            .send()
            .expect("Failed to send search request");

        assert_eq!(
            response.status(),
            200,
            "Search failed with status {}",
            response.status()
        );
        let body: serde_json::Value = response.json().expect("Failed to parse search response JSON");
        assert!(body.get("hits").is_some(), "Expected search response to contain 'hits'");
        assert!(
            body.get("taskUid").is_none(),
            "Search response should never contain a taskUid"
        );
    }
}
