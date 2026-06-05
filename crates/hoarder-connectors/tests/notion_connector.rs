use axum::{
    Json, Router,
    extract::{Query, State},
    http::HeaderMap,
    routing::{get, post},
};
use futures::StreamExt;
use hoarder_connectors::{
    notion::NotionSourceConnector,
    traits::{ConnectorConfig, SourceConnector},
};
use hoarder_core::types::{ConnectorKind, ItemType, SourceId};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc};
use tokio::net::TcpListener;

#[tokio::test]
async fn notion_connector_scans_data_source_page_and_reads_page_document() {
    let server = TestServer::start().await;
    let connector = NotionSourceConnector::new(SourceId::from_i64(7));
    let config = ConnectorConfig::Notion {
        token: "secret-token".to_owned(),
        data_source_id: Some("ds_123".to_owned()),
        page_id: None,
        version: Some("2026-03-11".to_owned()),
        base_url: Some(server.base_url.clone()),
    };

    let capabilities = connector.validate(&config).await.unwrap();
    assert_eq!(connector.kind(), ConnectorKind::Notion);
    assert!(capabilities.supports_virtual_documents);
    assert!(capabilities.supports_incremental_scan);

    let scan = connector.scan(&config, Some("cursor-1")).await.unwrap();
    assert_eq!(scan.next_cursor, Some("cursor-2".to_owned()));
    let items = scan.items.collect::<Vec<_>>().await;
    assert_eq!(items.len(), 1);
    let snapshot = items.into_iter().next().unwrap().unwrap();
    assert_eq!(snapshot.source_id, SourceId::from_i64(7));
    assert_eq!(snapshot.source_path, "notion/page/page_1.json");
    assert_eq!(snapshot.item_type, ItemType::VirtualDocument);
    assert_eq!(snapshot.etag, Some("2026-05-28T10:30:00.000Z".to_owned()));

    let item_ref = snapshot.item_ref();
    let mut bytes = connector.read(&config, &item_ref).await.unwrap();
    let mut body = Vec::new();
    while let Some(chunk) = bytes.next().await {
        body.extend_from_slice(&chunk.unwrap());
    }
    let document: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(document["provider"], json!("notion"));
    assert_eq!(document["page"]["id"], json!("page_1"));
    assert_eq!(document["children"][0]["type"], json!("paragraph"));
}

#[derive(Clone)]
struct TestState {
    auth_header: Arc<String>,
    notion_version: Arc<String>,
}

struct TestServer {
    base_url: String,
}

impl TestServer {
    async fn start() -> Self {
        let state = TestState {
            auth_header: Arc::new("Bearer secret-token".to_owned()),
            notion_version: Arc::new("2026-03-11".to_owned()),
        };
        let app = Router::new()
            .route("/v1/users/me", get(users_me))
            .route("/v1/data_sources/ds_123/query", post(query_data_source))
            .route("/v1/pages/page_1", get(get_page))
            .route("/v1/blocks/page_1/children", get(block_children))
            .with_state(state);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        Self {
            base_url: format!("http://{addr}"),
        }
    }
}

async fn users_me(State(state): State<TestState>, headers: HeaderMap) -> Json<Value> {
    assert_notion_headers(&headers, &state);
    Json(json!({"object":"user","id":"bot_1","type":"bot"}))
}

async fn query_data_source(
    State(state): State<TestState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Json<Value> {
    assert_notion_headers(&headers, &state);
    assert_eq!(body["page_size"], json!(100));
    assert_eq!(body["start_cursor"], json!("cursor-1"));

    Json(json!({
        "object": "list",
        "results": [{
            "object": "page",
            "id": "page_1",
            "last_edited_time": "2026-05-28T10:30:00.000Z",
            "url": "https://notion.so/page_1",
            "properties": {"Name": {"type": "title"}}
        }],
        "has_more": true,
        "next_cursor": "cursor-2"
    }))
}

async fn get_page(State(state): State<TestState>, headers: HeaderMap) -> Json<Value> {
    assert_notion_headers(&headers, &state);
    Json(json!({
        "object": "page",
        "id": "page_1",
        "last_edited_time": "2026-05-28T10:30:00.000Z",
        "properties": {"Name": {"type": "title"}}
    }))
}

async fn block_children(
    State(state): State<TestState>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Json<Value> {
    assert_notion_headers(&headers, &state);
    assert_eq!(query.get("page_size").map(String::as_str), Some("100"));
    Json(json!({
        "object": "list",
        "results": [{
            "object": "block",
            "id": "block_1",
            "type": "paragraph",
            "paragraph": {"rich_text": [{"plain_text": "hello"}]}
        }],
        "has_more": false,
        "next_cursor": null
    }))
}

fn assert_notion_headers(headers: &HeaderMap, state: &TestState) {
    assert_eq!(
        headers
            .get("authorization")
            .and_then(|value| value.to_str().ok()),
        Some(state.auth_header.as_str())
    );
    assert_eq!(
        headers
            .get("notion-version")
            .and_then(|value| value.to_str().ok()),
        Some(state.notion_version.as_str())
    );
}
