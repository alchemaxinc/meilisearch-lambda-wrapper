mod common;

use common::{IndexListResponse, TaskListResponse, TaskResponse, TestContext};
use std::{thread, time};

const MAX_POLL_ATTEMPTS: u32 = 30;
const POLL_INTERVAL_MS: u64 = 500;

fn step_1_seed_indexes(ctx: &TestContext) {
    let csv_data = include_bytes!("fixtures/movies.csv");

    let response = ctx
        .post("/indexes/movies/documents?primaryKey=id")
        .header("Content-Type", "text/csv")
        .body(csv_data.as_slice())
        .send()
        .expect("Failed to send seed request");

    assert_eq!(
        response.status().as_u16(),
        200,
        "Seed request failed with status {}",
        response.status()
    );
}

fn step_2_poll_task_by_id(ctx: &TestContext) {
    let poll_interval = time::Duration::from_millis(POLL_INTERVAL_MS);

    for attempt in 1..=MAX_POLL_ATTEMPTS {
        let response = ctx
            .get("/tasks/0")
            .send()
            .expect("Failed to send task poll request");

        if response.status() != 200 {
            eprintln!(
                "Attempt {attempt}/{MAX_POLL_ATTEMPTS}: Task endpoint returned status {}, waiting...",
                response.status()
            );
            thread::sleep(poll_interval);
            continue;
        }

        let task: TaskResponse = response.json().expect("Failed to parse task response JSON");

        if task.status != "succeeded" && task.status != "failed" {
            eprintln!(
                "Attempt {attempt}/{MAX_POLL_ATTEMPTS}: Task status is '{}', waiting...",
                task.status
            );
            thread::sleep(poll_interval);
            continue;
        }

        let details = task.details.expect("Task response missing 'details' field");
        assert_eq!(
            details.received_documents,
            Some(31944),
            "Unexpected receivedDocuments count"
        );
        assert_eq!(
            details.indexed_documents,
            Some(31944),
            "Unexpected indexedDocuments count"
        );
        return;
    }

    panic!("Task did not complete within {MAX_POLL_ATTEMPTS} attempts");
}

fn step_3_get_indexes(ctx: &TestContext) {
    let response = ctx
        .get("/indexes")
        .send()
        .expect("Failed to send get indexes request");

    assert_eq!(
        response.status().as_u16(),
        200,
        "Get indexes failed with status {}",
        response.status()
    );

    let data: IndexListResponse = response
        .json()
        .expect("Failed to parse indexes response JSON");

    assert_eq!(data.total, 1);
    assert_eq!(data.offset, 0);
    assert_eq!(data.limit, 20);
    assert_eq!(data.results.len(), 1);
    assert_eq!(data.results[0].uid, "movies");
    assert_eq!(data.results[0].primary_key, Some("id".to_string()));
}

fn step_4_get_all_tasks(ctx: &TestContext) {
    let response = ctx
        .get("/tasks")
        .send()
        .expect("Failed to send get tasks request");

    assert_eq!(
        response.status().as_u16(),
        200,
        "Get tasks failed with status {}",
        response.status()
    );

    let data: TaskListResponse = response
        .json()
        .expect("Failed to parse tasks response JSON");

    assert!(!data.results.is_empty(), "Expected at least one task");

    let task = &data.results[0];
    assert_eq!(task.index_uid, Some("movies".to_string()));
    assert_eq!(task.status, "succeeded");
    assert_eq!(task.task_type, "documentAdditionOrUpdate");
    assert_eq!(task.canceled_by, None);
    assert_eq!(task.error, None);

    let details = task.details.as_ref().expect("Task missing 'details' field");
    assert_eq!(details.received_documents, Some(31944));
    assert_eq!(details.indexed_documents, Some(31944));
}

#[test]
#[ignore]
fn test_meilisearch_integration() {
    let ctx = TestContext::new();

    step_1_seed_indexes(&ctx);
    step_2_poll_task_by_id(&ctx);
    step_3_get_indexes(&ctx);
    step_4_get_all_tasks(&ctx);
}
