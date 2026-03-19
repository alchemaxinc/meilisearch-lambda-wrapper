use reqwest::{blocking, header};
use serde::Deserialize;
use std::env;

pub struct TestContext {
    base_url: String,
    client: blocking::Client,
    headers: header::HeaderMap,
}

impl TestContext {
    pub fn new() -> Self {
        let host = env::var("MEILI_HOST").expect("MEILI_HOST environment variable is not set");
        let port = env::var("MEILI_PORT").expect("MEILI_PORT environment variable is not set");
        let master_key =
            env::var("MEILI_MASTER_KEY").expect("MEILI_MASTER_KEY environment variable is not set");

        let mut headers = header::HeaderMap::new();
        headers.insert(
            "Authorization",
            format!("Bearer {master_key}").parse().unwrap(),
        );

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
}

#[derive(Debug, Deserialize)]
pub struct TaskDetails {
    #[serde(rename = "receivedDocuments")]
    pub received_documents: Option<u64>,
    #[serde(rename = "indexedDocuments")]
    pub indexed_documents: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct TaskResponse {
    pub status: String,
    pub details: Option<TaskDetails>,
}

#[derive(Debug, Deserialize)]
pub struct IndexEntry {
    pub uid: String,
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
    pub canceled_by: Option<serde_json::Value>,
    pub details: Option<TaskDetails>,
    pub error: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct TaskListResponse {
    pub results: Vec<TaskEntry>,
}
