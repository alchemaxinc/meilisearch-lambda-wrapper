use std::env;

use reqwest::blocking;
use reqwest::header;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TaskDetails {
    #[serde(rename = "receivedDocuments")]
    pub received_documents: u64,
    #[serde(rename = "indexedDocuments")]
    pub indexed_documents: u64,
}

#[derive(Debug, Deserialize)]
pub struct TaskResponse {
    pub status: String,
    pub details: TaskDetails,
}

/// A minimal task response shape. Every async Meilisearch operation
/// returns this shape: index creation, settings updates, document
/// deletion, and task cancellation or deletion. Not every task type
/// returns a `details` field, so this struct omits it.
#[derive(Debug, Deserialize)]
pub struct SimpleTaskResponse {
    pub status: String,
    #[serde(rename = "type")]
    pub task_type: String,
}

#[derive(Debug, Deserialize)]
pub struct IndexEntry {
    pub uid: String,
    // Meilisearch's index listing can briefly show a null primaryKey right
    // after index creation, before the metadata catches up with the task.
    #[serde(rename = "primaryKey")]
    pub primary_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct IndexListResponse {
    pub results: Vec<IndexEntry>,
    pub offset: u64,
    pub limit: u64,
    pub total: u64,
}

#[derive(Debug, Deserialize)]
pub struct TaskEntry {
    #[serde(rename = "indexUid")]
    pub index_uid: Option<String>,
    pub status: String,
    #[serde(rename = "type")]
    pub task_type: String,
    #[serde(rename = "canceledBy")]
    pub canceled_by: serde_json::Value,
    // Not every task type has this shape, so parse it as raw JSON here
    // and read specific fields only where the task type is known.
    pub details: serde_json::Value,
    pub error: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct TaskListResponse {
    pub results: Vec<TaskEntry>,
}

#[derive(Debug, Deserialize)]
pub struct KeyListResponse {
    pub results: Vec<serde_json::Value>,
}

pub struct TestContext {
    base_url: String,
    client: blocking::Client,
    headers: header::HeaderMap,
}

impl TestContext {
    pub fn new() -> Self {
        let port = env::var("MEILI_PORT").unwrap_or_else(|_| return "8080".to_string());
        // Since running this locally, you want to use `localhost`, but in a docker-compose'd network,
        // we need to overwrite it with the docker container's hostname.
        let host = env::var("MEILI_HOST").unwrap_or_else(|_| return "localhost".to_string());
        let master_key = env::var("MEILI_MASTER_KEY").expect("MEILI_MASTER_KEY environment variable is not set");

        let mut headers = header::HeaderMap::new();
        headers.insert("Authorization", format!("Bearer {master_key}").parse().unwrap());

        return Self {
            client: blocking::Client::new(),
            base_url: format!("http://{host}:{port}"),
            headers,
        };
    }

    pub fn get(&self, path: &str) -> blocking::RequestBuilder {
        return self
            .client
            .get(format!("{}{}", self.base_url, path))
            .headers(self.headers.clone());
    }

    pub fn post(&self, path: &str) -> blocking::RequestBuilder {
        return self
            .client
            .post(format!("{}{}", self.base_url, path))
            .headers(self.headers.clone());
    }

    pub fn put(&self, path: &str) -> blocking::RequestBuilder {
        return self
            .client
            .put(format!("{}{}", self.base_url, path))
            .headers(self.headers.clone());
    }

    pub fn patch(&self, path: &str) -> blocking::RequestBuilder {
        return self
            .client
            .patch(format!("{}{}", self.base_url, path))
            .headers(self.headers.clone());
    }

    pub fn delete(&self, path: &str) -> blocking::RequestBuilder {
        return self
            .client
            .delete(format!("{}{}", self.base_url, path))
            .headers(self.headers.clone());
    }
}
