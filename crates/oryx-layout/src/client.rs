//! Fetches a layout revision from Oryx's GraphQL API.

use std::time::Duration;

use oryx_protocol::LayoutIds;
use serde_json::{Value, json};

pub use reqwest::Client as HttpClient;

pub const ORYX_GRAPHQL_URL: &str = "https://oryx.zsa.io/graphql";
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

const QUERY: &str = "query getLayout($hashId: String!, $revisionId: String!, $geometry: String) { \
layout(hashId: $hashId, geometry: $geometry, revisionId: $revisionId) { \
title geometry hashId revision { hashId layers { position title keys } } } }";

#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Oryx returned HTTP {0}")]
    Status(u16),
}

pub fn user_agent(version: &str) -> String {
    format!("Downlink/{version} (+https://github.com/zayatura/downlink)")
}

pub fn client_builder(version: &str) -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .user_agent(user_agent(version))
        .timeout(REQUEST_TIMEOUT)
}

pub fn request_body(ids: &LayoutIds, geometry: &str) -> Value {
    json!({
        "operationName": "getLayout",
        "query": QUERY,
        "variables": {"hashId": ids.layout_id(), "revisionId": ids.revision_id(), "geometry": geometry},
    })
}

/// Returns the raw JSON body; parse it with [`crate::parse_graphql`].
pub async fn fetch_layout(
    client: &HttpClient,
    url: &str,
    ids: &LayoutIds,
    geometry: &str,
) -> Result<String, FetchError> {
    let response = client
        .post(url)
        .json(&request_body(ids, geometry))
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        return Err(FetchError::Status(status.as_u16()));
    }
    Ok(response.text().await?)
}
