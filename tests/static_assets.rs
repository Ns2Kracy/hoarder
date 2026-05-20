use std::{fmt::Write as _, path::PathBuf};

use axum::Router;
use hoarder::{AppConfig, server};
use serde_json::json;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

#[tokio::test]
async fn static_assets_serves_app_shell_for_frontend_routes() {
    let root = request(test_app().await, "GET", "/").await;
    assert_eq!(root.status, 200);
    assert!(root.content_type().starts_with("text/html"));

    let response = request(test_app().await, "GET", "/sources").await;

    assert_eq!(response.status, 200);
    assert!(response.content_type().starts_with("text/html"));
    assert!(response.body_text().contains(r#"<div id="app"></div>"#));
}

#[tokio::test]
async fn static_assets_keep_unmatched_api_routes_json() {
    let response = request(test_app().await, "GET", "/api/missing").await;

    assert_eq!(response.status, 404);
    assert!(response.content_type().starts_with("application/json"));

    let body: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(
        body,
        json!({
            "error": {
                "code": "NOT_FOUND",
                "message": "API route not found"
            }
        })
    );
}

#[tokio::test]
async fn static_assets_server_sets_request_id_and_cors_headers() {
    let origin = "http://localhost:4173";
    let response = request_with_headers(
        test_app().await,
        "GET",
        "/api/health",
        &[("Origin", origin)],
    )
    .await;

    assert_eq!(response.status, 200);
    assert!(!response.header("x-request-id").is_empty());
    assert_eq!(response.header("access-control-allow-origin"), origin);
}

async fn test_app() -> Router {
    let config = AppConfig {
        database_path: PathBuf::from(":memory:"),
        vault_path: PathBuf::from("./target/test-vault"),
        ..AppConfig::default()
    };
    server::app(config).await.unwrap()
}

struct HttpResponse {
    status: u16,
    headers: String,
    body: Vec<u8>,
}

impl HttpResponse {
    fn header(&self, name: &str) -> &str {
        self.headers
            .lines()
            .find_map(|line| {
                line.split_once(':').and_then(|(header_name, value)| {
                    header_name
                        .eq_ignore_ascii_case(name)
                        .then_some(value.trim())
                })
            })
            .unwrap_or("")
    }

    fn content_type(&self) -> &str {
        self.header("content-type")
    }

    fn body_text(&self) -> String {
        String::from_utf8(self.body.clone()).unwrap()
    }
}

async fn request(app: Router, method: &str, path: &str) -> HttpResponse {
    request_with_headers(app, method, path, &[]).await
}

async fn request_with_headers(
    app: Router,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
) -> HttpResponse {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let mut stream = TcpStream::connect(addr).await.unwrap();
    let mut headers_text = String::new();
    for (name, value) in headers {
        write!(&mut headers_text, "{name}: {value}\r\n").unwrap();
    }
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n{headers_text}\r\n"
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
    let headers = String::from_utf8_lossy(&response[..separator]).to_string();
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
        headers,
        body,
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
