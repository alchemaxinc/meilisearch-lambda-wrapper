use reqwest::{blocking, header};
use serde::Deserialize;
use std::env;

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

#[derive(Debug, Deserialize)]
pub struct IndexEntry {
    pub uid: String,
    #[serde(rename = "primaryKey")]
    pub primary_key: String,
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
    pub index_uid: String,
    pub status: String,
    #[serde(rename = "type")]
    pub task_type: String,
    #[serde(rename = "canceledBy")]
    pub canceled_by: serde_json::Value,
    pub details: TaskDetails,
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

const MEILISEARCH_HOST: &str = "http://localhost:8080";

impl TestContext {
    pub fn new() -> Self {
        let master_key =
            env::var("MEILI_MASTER_KEY").expect("MEILI_MASTER_KEY environment variable is not set");

        let mut headers = header::HeaderMap::new();
        headers.insert(
            "Authorization",
            format!("Bearer {master_key}").parse().unwrap(),
        );

        return Self {
            client: blocking::Client::new(),
            base_url: MEILISEARCH_HOST.to_string(),
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
