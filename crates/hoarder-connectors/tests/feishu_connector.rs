use axum::{
    Json, Router,
    extract::{Query, State},
    http::HeaderMap,
    routing::{get, post},
};
use futures::StreamExt;
use hoarder_connectors::{
    feishu::FeishuSourceConnector,
    traits::{ConnectorConfig, SourceConnector},
};
use hoarder_core::types::{ConnectorKind, ItemType, SourceId};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc};
use tokio::net::TcpListener;

#[tokio::test]
async fn feishu_connector_scans_folder_with_page_token_and_reads_reference_document() {
    let server = TestServer::start().await;
    let connector = FeishuSourceConnector::new(SourceId::from_i64(9));
    let config = ConnectorConfig::Feishu {
        app_id: "cli_app".to_owned(),
        app_secret: "app-secret".to_owned(),
        folder_token: Some("folder_123".to_owned()),
        base_url: Some(server.base_url.clone()),
    };

    let capabilities = connector.validate(&config).await.unwrap();
    assert_eq!(connector.kind(), ConnectorKind::Feishu);
    assert!(capabilities.supports_virtual_documents);
    assert!(capabilities.supports_incremental_scan);

    let scan = connector.scan(&config, Some("page-1")).await.unwrap();
    assert_eq!(scan.next_cursor, Some("page-2".to_owned()));
    let items = scan.items.collect::<Vec<_>>().await;
    assert_eq!(items.len(), 1);
    let snapshot = items.into_iter().next().unwrap().unwrap();
    assert_eq!(snapshot.source_id, SourceId::from_i64(9));
    assert_eq!(snapshot.source_path, "feishu/docx/docx_token.json");
    assert_eq!(snapshot.item_type, ItemType::VirtualDocument);
    assert_eq!(snapshot.etag, Some("1780000000".to_owned()));
    assert_eq!(
        snapshot.metadata_json.as_ref().unwrap()["name"],
        json!("Runbook")
    );

    let item_ref = snapshot.item_ref();
    let mut bytes = connector.read(&config, &item_ref).await.unwrap();
    let mut body = Vec::new();
    while let Some(chunk) = bytes.next().await {
        body.extend_from_slice(&chunk.unwrap());
    }
    let document: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(document["provider"], json!("feishu"));
    assert_eq!(document["token"], json!("docx_token"));
    assert_eq!(document["sourcePath"], json!("feishu/docx/docx_token.json"));
}

#[derive(Clone)]
struct TestState {
    bearer: Arc<String>,
}

struct TestServer {
    base_url: String,
}

impl TestServer {
    async fn start() -> Self {
        let app = Router::new()
            .route(
                "/open-apis/auth/v3/tenant_access_token/internal",
                post(tenant_access_token),
            )
            .route("/open-apis/drive/v1/files", get(list_files))
            .with_state(TestState {
                bearer: Arc::new("Bearer tenant-token".to_owned()),
            });
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

async fn tenant_access_token(Json(body): Json<Value>) -> Json<Value> {
    assert_eq!(body["app_id"], json!("cli_app"));
    assert_eq!(body["app_secret"], json!("app-secret"));
    Json(json!({
        "code": 0,
        "msg": "ok",
        "tenant_access_token": "tenant-token",
        "expire": 7200
    }))
}

async fn list_files(
    State(state): State<TestState>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Json<Value> {
    assert_eq!(
        headers
            .get("authorization")
            .and_then(|value| value.to_str().ok()),
        Some(state.bearer.as_str())
    );
    assert_eq!(
        query.get("folder_token").map(String::as_str),
        Some("folder_123")
    );
    assert_eq!(query.get("page_size").map(String::as_str), Some("50"));
    assert_eq!(query.get("page_token").map(String::as_str), Some("page-1"));

    Json(json!({
        "code": 0,
        "msg": "ok",
        "data": {
            "files": [{
                "token": "docx_token",
                "name": "Runbook",
                "type": "docx",
                "url": "https://feishu.cn/docx/docx_token",
                "size": "2048",
                "modified_time": "1780000000"
            }],
            "has_more": true,
            "next_page_token": "page-2"
        }
    }))
}
