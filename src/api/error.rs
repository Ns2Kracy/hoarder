use axum::{
    Json,
    extract::rejection::{JsonRejection, PathRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::error::AppError;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiErrorBody {
    pub error: ApiErrorDetail,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiErrorDetail {
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    body: ApiErrorBody,
}

impl ApiError {
    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "VALIDATION_ERROR", message.into())
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, "NOT_FOUND", message.into())
    }

    fn new(status: StatusCode, code: &'static str, message: String) -> Self {
        Self {
            status,
            body: ApiErrorBody {
                error: ApiErrorDetail { code, message },
            },
        }
    }
}

impl From<AppError> for ApiError {
    fn from(error: AppError) -> Self {
        match error {
            AppError::Config(message) => {
                Self::new(StatusCode::BAD_REQUEST, "CONFIG_ERROR", message)
            }
            AppError::Connector(message) => {
                Self::new(StatusCode::BAD_GATEWAY, "CONNECTOR_ERROR", message)
            }
            AppError::Io(_) => Self::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                "internal server error".to_owned(),
            ),
            AppError::NotFound(message) => Self::not_found(message),
            AppError::Path(message) => Self::new(StatusCode::BAD_REQUEST, "PATH_ERROR", message),
            AppError::Validation(message) => Self::validation(message),
        }
    }
}

impl From<JsonRejection> for ApiError {
    fn from(rejection: JsonRejection) -> Self {
        Self::validation(format!("invalid JSON body: {rejection}"))
    }
}

impl From<PathRejection> for ApiError {
    fn from(rejection: PathRejection) -> Self {
        Self::validation(format!("invalid path parameter: {rejection}"))
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}
