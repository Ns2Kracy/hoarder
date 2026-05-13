use axum::{body::to_bytes, http::StatusCode, response::IntoResponse};
use hoarder::{
    api::{error::ApiError, types::SourceDto},
    connectors::traits::ConnectorConfig,
    core::types::SourceId,
    error::AppError,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use uuid::Uuid;

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
    let source_id =
        SourceId::from_uuid(Uuid::parse_str("018f3f55-6b4d-7b2f-8b1e-f7563f31b8d5").unwrap());
    let config = ConnectorConfig::OpenDal {
        service: "s3".to_owned(),
        options: BTreeMap::from([
            ("bucket".to_owned(), "docs".to_owned()),
            ("access_key_id".to_owned(), "AKIASECRET".to_owned()),
            ("secret_access_key".to_owned(), "very-secret".to_owned()),
            ("session_token".to_owned(), "token-value".to_owned()),
        ]),
    };

    let source = SourceDto::new(
        source_id,
        "Docs".to_owned(),
        &config,
        true,
        hoarder::api::types::SourceHealth::Untested,
        None,
    );
    let encoded = serde_json::to_value(source).unwrap();

    assert_eq!(encoded["config"]["options"]["bucket"], json!("docs"));
    assert_eq!(
        encoded["config"]["options"]["access_key_id"],
        json!("<redacted>")
    );
    assert!(!encoded.to_string().contains("AKIASECRET"));
    assert!(!encoded.to_string().contains("very-secret"));
    assert!(!encoded.to_string().contains("token-value"));
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}
