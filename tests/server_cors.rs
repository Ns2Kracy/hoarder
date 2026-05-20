use std::path::PathBuf;

use axum::Router;
use hoarder::{AppConfig, db::connect_sqlite, server};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

#[tokio::test]
async fn server_cors_allows_embedded_console_and_local_vite_origins() {
    let app = test_app().await;

    for origin in ["http://127.0.0.1:4761", "http://localhost:5173"] {
        let response = raw_request(
            app.clone(),
            &format!(
                "OPTIONS /api/settings HTTP/1.1\r\n\
                 Host: 127.0.0.1:4761\r\n\
                 Origin: {origin}\r\n\
                 Access-Control-Request-Method: PATCH\r\n\
                 Access-Control-Request-Headers: content-type\r\n\
                 Connection: close\r\n\r\n"
            ),
        )
        .await;

        assert_eq!(response.status, 200);
        assert_eq!(response.header("access-control-allow-origin"), Some(origin));
        assert!(
            response
                .header("access-control-allow-methods")
                .is_some_and(|methods| methods.contains("PATCH"))
        );
        assert!(
            response
                .header("access-control-allow-headers")
                .is_some_and(|headers| headers.eq_ignore_ascii_case("content-type"))
        );
    }
}

#[tokio::test]
async fn server_cors_does_not_permissively_allow_unrelated_origins() {
    let response = raw_request(
        test_app().await,
        "OPTIONS /api/settings HTTP/1.1\r\n\
         Host: 127.0.0.1:4761\r\n\
         Origin: https://example.com\r\n\
         Access-Control-Request-Method: PATCH\r\n\
         Access-Control-Request-Headers: content-type\r\n\
         Connection: close\r\n\r\n",
    )
    .await;

    assert_eq!(response.status, 200);
    assert_ne!(
        response.header("access-control-allow-origin"),
        Some("*"),
        "unrelated origins must not receive wildcard CORS access"
    );
    assert!(
        response.header("access-control-allow-origin").is_none(),
        "unrelated origins should not receive an allow-origin header"
    );
}

async fn test_app() -> Router {
    let db = connect_sqlite("sqlite::memory:").await.unwrap();
    let config = AppConfig {
        database_path: PathBuf::from(":memory:"),
        ..AppConfig::default()
    };

    server::app_with_db(config, db)
}

async fn raw_request(app: Router, request: &str) -> HttpResponse {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let mut stream = TcpStream::connect(addr).await.unwrap();
    stream.write_all(request.as_bytes()).await.unwrap();

    let mut response = Vec::new();
    stream.read_to_end(&mut response).await.unwrap();
    server.abort();

    decode_response(&response)
}

#[derive(Clone, Debug)]
struct HttpResponse {
    status: u16,
    headers: Vec<(String, String)>,
}

impl HttpResponse {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(candidate, _)| candidate.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

fn decode_response(response: &[u8]) -> HttpResponse {
    let separator = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .expect("response contains header separator");
    let headers = String::from_utf8_lossy(&response[..separator]);
    let mut lines = headers.lines();
    let status = lines
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|status| status.parse::<u16>().ok())
        .expect("response contains status");
    let headers = lines
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            Some((name.trim().to_owned(), value.trim().to_owned()))
        })
        .collect();

    HttpResponse { status, headers }
}
