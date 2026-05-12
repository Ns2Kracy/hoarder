use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use axum::Router;
use futures::FutureExt;
use hoarder::{
    api::{
        routes::router,
        state::{ApiFuture, ApiRepository, ApiState, SyncService},
        types::{
            CreateSourceRequest, ItemDto, JobDto, JobRunResponse, RunDto, SettingsDto, SourceDto,
            SyncErrorDto,
        },
    },
    config::AppConfig,
    connectors::traits::ConnectorConfig,
    core::types::{JobId, RunId, SourceId, SyncStatus},
};
use serde_json::{Value, json};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use uuid::Uuid;

#[tokio::test]
async fn api_routes_health_returns_success() {
    let response = request(test_router(), "GET", "/api/health", None).await;

    assert_eq!(response.status, 200);
    assert_eq!(response.body, json!({ "status": "ok" }));
}

#[tokio::test]
async fn api_routes_sources_returns_repository_list() {
    let response = request(test_router(), "GET", "/api/sources", None).await;

    assert_eq!(response.status, 200);
    assert_eq!(response.body["data"][0]["name"], json!("Local Docs"));
    assert_eq!(
        response.body["data"][0]["config"]["options"]["root"],
        json!(".")
    );
}

#[tokio::test]
async fn api_routes_collection_endpoints_return_lists_and_settings() {
    for path in ["/api/jobs", "/api/runs", "/api/items", "/api/errors"] {
        let response = request(test_router(), "GET", path, None).await;

        assert_eq!(response.status, 200, "{path}");
        assert_eq!(response.body, json!({ "data": [] }), "{path}");
    }

    let settings = request(test_router(), "GET", "/api/settings", None).await;
    assert_eq!(settings.status, 200);
    assert_eq!(settings.body["listenAddr"], json!("127.0.0.1:4761"));
}

#[tokio::test]
async fn api_routes_run_job_triggers_sync_service() {
    let job_id = JobId::from_uuid(Uuid::parse_str("018f3f55-6b4d-7b2f-8b1e-f7563f31b8d5").unwrap());
    let sync = Arc::new(RecordingSyncService::default());
    let state = ApiState::new(Arc::new(FakeRepository::default()), sync.clone());
    let app = router(state);

    let response = request(app, "POST", &format!("/api/jobs/{job_id}/run"), Some("")).await;

    assert_eq!(response.status, 200);
    assert_eq!(response.body["status"], json!("pending"));
    assert_eq!(sync.recorded_job_ids(), vec![job_id]);
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

    loop {
        let Some(size_end) = body.windows(2).position(|window| window == b"\r\n") else {
            break;
        };
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

fn test_router() -> Router {
    let state = ApiState::new(
        Arc::new(FakeRepository::default()),
        Arc::new(RecordingSyncService::default()),
    );

    router(state)
}

#[derive(Default)]
struct FakeRepository;

impl ApiRepository for FakeRepository {
    fn list_sources(&self) -> ApiFuture<'_, Vec<SourceDto>> {
        async move {
            let source_id = SourceId::from_uuid(
                Uuid::parse_str("018f3f55-6b4d-7b2f-8b1e-f7563f31b8d5").unwrap(),
            );
            let config = ConnectorConfig::OpenDal {
                service: "fs".to_owned(),
                options: BTreeMap::from([("root".to_owned(), ".".to_owned())]),
            };

            Ok(vec![SourceDto::new(
                source_id,
                "Local Docs".to_owned(),
                &config,
                true,
            )])
        }
        .boxed()
    }

    fn create_source(&self, request: CreateSourceRequest) -> ApiFuture<'_, SourceDto> {
        async move {
            Ok(SourceDto::new(
                SourceId::new(),
                request.name,
                &request.config,
                request.enabled,
            ))
        }
        .boxed()
    }

    fn list_jobs(&self) -> ApiFuture<'_, Vec<JobDto>> {
        async move { Ok(vec![]) }.boxed()
    }

    fn list_runs(&self) -> ApiFuture<'_, Vec<RunDto>> {
        async move { Ok(vec![]) }.boxed()
    }

    fn list_items(&self) -> ApiFuture<'_, Vec<ItemDto>> {
        async move { Ok(vec![]) }.boxed()
    }

    fn list_errors(&self) -> ApiFuture<'_, Vec<SyncErrorDto>> {
        async move { Ok(vec![]) }.boxed()
    }

    fn settings(&self) -> ApiFuture<'_, SettingsDto> {
        async move { Ok(SettingsDto::from(&AppConfig::default())) }.boxed()
    }
}

#[derive(Default)]
struct RecordingSyncService {
    job_ids: Mutex<Vec<JobId>>,
}

impl RecordingSyncService {
    fn recorded_job_ids(&self) -> Vec<JobId> {
        self.job_ids.lock().unwrap().clone()
    }
}

impl SyncService for RecordingSyncService {
    fn run_job(&self, job_id: JobId) -> ApiFuture<'_, JobRunResponse> {
        async move {
            self.job_ids.lock().unwrap().push(job_id);

            Ok(JobRunResponse {
                run_id: RunId::new(),
                status: SyncStatus::Pending,
            })
        }
        .boxed()
    }
}
