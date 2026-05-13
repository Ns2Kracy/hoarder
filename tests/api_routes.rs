use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use axum::Router;
use hoarder::{
    api::{routes::router, state::ApiState},
    config::AppConfig,
    connectors::traits::ConnectorConfig,
    core::types::{ConnectorKind, JobId, SourceId},
    db::{
        connect_sqlite,
        repository::{
            NewSource, NewSyncJob, SeaOrmRepository, SourceRepository, SyncJobRepository,
        },
        schema::sync_schema,
    },
};
use serde_json::{Value, json};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use uuid::Uuid;

#[tokio::test]
async fn api_routes_health_returns_success() {
    let test = TestApp::new().await;
    let response = request(test.app.clone(), "GET", "/api/health", None).await;

    assert_eq!(response.status, 200);
    assert_eq!(response.body, json!({ "status": "ok" }));
}

#[tokio::test]
async fn api_routes_sources_returns_repository_list() {
    let test = TestApp::new().await;
    let response = request(test.app.clone(), "GET", "/api/sources", None).await;

    assert_eq!(response.status, 200);
    assert_eq!(response.body["data"][0]["name"], json!("Local Docs"));
    assert_eq!(response.body["data"][0]["health"], json!("untested"));
    assert_eq!(
        response.body["data"][0]["config"]["options"]["root"],
        json!(test.source_root.to_string_lossy())
    );
}

#[tokio::test]
async fn api_routes_collection_endpoints_return_lists_and_settings() {
    let test = TestApp::new().await;
    let jobs = request(test.app.clone(), "GET", "/api/jobs", None).await;

    assert_eq!(jobs.status, 200);
    assert_eq!(jobs.body["data"][0]["id"], json!(test.job_id.to_string()));

    for path in ["/api/runs", "/api/items", "/api/errors"] {
        let response = request(test.app.clone(), "GET", path, None).await;

        assert_eq!(response.status, 200, "{path}");
        assert_eq!(response.body, json!({ "data": [] }), "{path}");
    }

    let settings = request(test.app.clone(), "GET", "/api/settings", None).await;
    assert_eq!(settings.status, 200);
    assert_eq!(settings.body["listenAddr"], json!("127.0.0.1:4761"));
}

#[tokio::test]
async fn api_routes_run_job_runs_sync_engine() {
    let test = TestApp::new().await;

    let response = request(
        test.app.clone(),
        "POST",
        &format!("/api/jobs/{}/run", test.job_id),
        Some(""),
    )
    .await;

    assert_eq!(response.status, 200);
    assert_eq!(response.body["status"], json!("synced"));
    assert!(response.body["runId"].as_str().is_some());
}

#[tokio::test]
async fn api_routes_test_source_checks_repository() {
    let test = TestApp::new().await;
    let response = request(
        test.app.clone(),
        "POST",
        &format!("/api/sources/{}/test", test.source_id),
        Some(""),
    )
    .await;

    assert_eq!(response.status, 200);
    assert_eq!(response.body["ok"], json!(true));
    assert!(response.body["checkedAt"].as_str().is_some());

    let sources = request(test.app.clone(), "GET", "/api/sources", None).await;
    assert_eq!(sources.status, 200);
    assert_eq!(sources.body["data"][0]["health"], json!("healthy"));
    assert!(sources.body["data"][0]["lastCheckedAt"].as_str().is_some());
}

#[derive(Clone)]
struct HttpResponse {
    status: u16,
    body: Value,
}

async fn request(app: Router, method: &str, path: &str, body: Option<&str>) -> HttpResponse {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let body = body.unwrap_or("");
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).await.unwrap();

    let mut response = Vec::new();
    stream.read_to_end(&mut response).await.unwrap();
    server.abort();

    decode_response(&response)
}

fn decode_response(response: &[u8]) -> HttpResponse {
    let separator = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .expect("response contains header separator");
    let headers = String::from_utf8_lossy(&response[..separator]);
    let body = &response[separator + 4..];
    let status = headers
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|status| status.parse::<u16>().ok())
        .expect("response contains status");
    let body = if headers
        .to_ascii_lowercase()
        .contains("transfer-encoding: chunked")
    {
        decode_chunked(body)
    } else {
        body.to_vec()
    };

    HttpResponse {
        status,
        body: serde_json::from_slice(&body).unwrap(),
    }
}

fn decode_chunked(mut body: &[u8]) -> Vec<u8> {
    let mut decoded = Vec::new();

    while let Some(size_end) = body.windows(2).position(|window| window == b"\r\n") {
        let size = std::str::from_utf8(&body[..size_end]).unwrap();
        let size = usize::from_str_radix(size.trim(), 16).unwrap();
        if size == 0 {
            break;
        }

        let chunk_start = size_end + 2;
        let chunk_end = chunk_start + size;
        decoded.extend_from_slice(&body[chunk_start..chunk_end]);
        body = &body[chunk_end + 2..];
    }

    decoded
}

struct TestApp {
    app: Router,
    source_id: SourceId,
    job_id: JobId,
    source_root: PathBuf,
    _temp: TempDir,
}

impl TestApp {
    async fn new() -> Self {
        let temp = TempDir::new("api-routes");
        let source_root = temp.path.join("source");
        let vault_root = temp.path.join("vault");
        fs::create_dir_all(&source_root).unwrap();
        fs::write(source_root.join("readme.md"), "hello").unwrap();

        let db = connect_sqlite("sqlite::memory:").await.unwrap();
        sync_schema(&db).await.unwrap();
        let repository = Arc::new(SeaOrmRepository::new(db));
        let source_config = fs_config(&source_root);
        let source = repository
            .create_source(NewSource {
                name: "Local Docs".to_owned(),
                kind: ConnectorKind::OpenDal,
                config_json: serde_json::to_value(&source_config).unwrap(),
                enabled: true,
            })
            .await
            .unwrap();
        let job = repository
            .create_job(NewSyncJob {
                source_id: source.id,
                name: "Default sync".to_owned(),
                enabled: true,
            })
            .await
            .unwrap();
        let config = AppConfig {
            database_path: PathBuf::from(":memory:"),
            vault_path: vault_root,
            ..AppConfig::default()
        };
        let state = ApiState::new(repository, config);

        Self {
            app: router(state),
            source_id: source.id,
            job_id: job.id,
            source_root,
            _temp: temp,
        }
    }
}

fn fs_config(root: &Path) -> ConnectorConfig {
    ConnectorConfig::OpenDal {
        service: "fs".to_owned(),
        options: BTreeMap::from([("root".to_owned(), root.to_string_lossy().into_owned())]),
    }
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("hoarder-{name}-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();

        Self { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
