use axum::{body::to_bytes, http::StatusCode, response::IntoResponse};
use hoarder_connectors::traits::ConnectorConfig;
use hoarder_core::{AppError, types::SourceId};
use hoarder_server::api::{error::ApiError, types::SourceDto};
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[tokio::test]
async fn api_error_serializes_validation_errors_with_stable_shape() {
    let response =
        ApiError::from(AppError::Validation("name is required".to_owned())).into_response();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response_json(response).await;

    assert_eq!(
        body,
        json!({
            "error": {
                "code": "VALIDATION_ERROR",
                "message": "name is required"
            }
        })
    );
}

#[tokio::test]
async fn api_error_serializes_not_found_errors() {
    let response = ApiError::from(AppError::NotFound("source missing".to_owned())).into_response();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = response_json(response).await;

    assert_eq!(body["error"]["code"], json!("NOT_FOUND"));
    assert_eq!(body["error"]["message"], json!("source missing"));
}

#[tokio::test]
async fn api_error_serializes_conflict_errors() {
    let response =
        ApiError::from(AppError::Conflict("job is already running".to_owned())).into_response();

    assert_eq!(response.status(), StatusCode::CONFLICT);
    let body = response_json(response).await;

    assert_eq!(body["error"]["code"], json!("CONFLICT"));
    assert_eq!(body["error"]["message"], json!("job is already running"));
}

#[tokio::test]
async fn api_error_serializes_unprocessable_errors() {
    let response = ApiError::from(AppError::Unprocessable(
        "interval must be positive".to_owned(),
    ))
    .into_response();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = response_json(response).await;

    assert_eq!(body["error"]["code"], json!("UNPROCESSABLE_ENTITY"));
    assert_eq!(body["error"]["message"], json!("interval must be positive"));
}

#[tokio::test]
async fn api_error_hides_internal_io_error_details() {
    let response = ApiError::from(AppError::Io(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "private filesystem detail",
    )))
    .into_response();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = response_json(response).await;

    assert_eq!(body["error"]["code"], json!("INTERNAL_ERROR"));
    assert_eq!(body["error"]["message"], json!("internal server error"));
}

#[tokio::test]
async fn api_error_hides_internal_database_error_details() {
    let response =
        ApiError::from(AppError::Database("private database detail".to_owned())).into_response();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = response_json(response).await;

    assert_eq!(body["error"]["code"], json!("INTERNAL_ERROR"));
    assert_eq!(body["error"]["message"], json!("internal server error"));
}

#[test]
fn api_error_source_dto_redacts_secret_config_values() {
    let source_id = SourceId::from_i64(42);
    let config = ConnectorConfig::OpenDal {
        service: "s3".to_owned(),
        options: BTreeMap::from([
            ("bucket".to_owned(), "docs".to_owned()),
            ("access_key_id".to_owned(), "AKIASECRET".to_owned()),
            ("secret_access_key".to_owned(), "very-secret".to_owned()),
            ("session_token".to_owned(), "token-value".to_owned()),
            ("private_key".to_owned(), "private-key-value".to_owned()),
            ("key".to_owned(), "key-value".to_owned()),
        ]),
    };

    let source = SourceDto::new(
        source_id,
        "Docs".to_owned(),
        &config,
        true,
        hoarder_server::api::types::SourceHealth::Untested,
        None,
    );
    let encoded = serde_json::to_value(source).unwrap();

    assert_eq!(encoded["config"]["options"]["bucket"], json!("docs"));
    assert_eq!(
        encoded["config"]["options"]["access_key_id"],
        json!("<redacted>")
    );
    assert_eq!(
        encoded["config"]["options"]["private_key"],
        json!("<redacted>")
    );
    assert_eq!(encoded["config"]["options"]["key"], json!("<redacted>"));
    assert!(!encoded.to_string().contains("AKIASECRET"));
    assert!(!encoded.to_string().contains("very-secret"));
    assert!(!encoded.to_string().contains("token-value"));
    assert!(!encoded.to_string().contains("private-key-value"));
    assert!(!encoded.to_string().contains("key-value"));
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}
