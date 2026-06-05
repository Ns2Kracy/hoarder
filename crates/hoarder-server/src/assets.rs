use axum::{
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
};
use rust_embed::RustEmbed;

use crate::api::error::ApiError;

const INDEX_HTML: &str = "index.html";

#[derive(RustEmbed)]
#[folder = "../../web/dist"]
struct WebAssets;

pub async fn serve(uri: Uri) -> Response {
    response_for_path(uri.path())
}

#[must_use]
pub fn response_for_path(path: &str) -> Response {
    let path = path.trim_start_matches('/');

    if path.is_empty() || path == INDEX_HTML {
        return index_html();
    }

    if let Some(file) = WebAssets::get(path) {
        return file_response(path, file);
    }

    if path == "api" || path.starts_with("api/") {
        return ApiError::not_found("API route not found").into_response();
    }

    if path.contains('.') {
        return not_found("asset not found");
    }

    index_html()
}

fn index_html() -> Response {
    WebAssets::get(INDEX_HTML).map_or_else(
        || not_found("frontend assets have not been built"),
        |file| file_response(INDEX_HTML, file),
    )
}

fn file_response(path: &str, file: rust_embed::EmbeddedFile) -> Response {
    let mime = mime_guess::from_path(path).first_or_octet_stream();

    ([(header::CONTENT_TYPE, mime.as_ref())], file.data).into_response()
}

fn not_found(message: &'static str) -> Response {
    (
        StatusCode::NOT_FOUND,
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        message,
    )
        .into_response()
}
