use axum::{
    body::Body,
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
};
use rust_embed::RustEmbed;

use crate::api::error::ApiError;

#[derive(RustEmbed)]
#[folder = "web/dist"]
struct WebAssets;

pub async fn serve(uri: Uri) -> Response {
    response_for_path(uri.path())
}

#[must_use]
pub fn response_for_path(path: &str) -> Response {
    let asset_path = path.trim_start_matches('/');

    if let Some((path, file)) = asset_file(asset_path) {
        return file_response(path, file);
    }

    if is_api_path(asset_path) {
        return ApiError::not_found("API route not found").into_response();
    }

    if asset_path.starts_with("assets/") {
        return not_found("asset not found");
    }

    WebAssets::get("index.html").map_or_else(
        || not_found("frontend assets have not been built"),
        |file| file_response("index.html", file),
    )
}

fn asset_file(path: &str) -> Option<(&str, rust_embed::EmbeddedFile)> {
    let path = match path {
        "" => "index.html",
        path => path,
    };

    WebAssets::get(path).map(|file| (path, file))
}

fn file_response(path: &str, file: rust_embed::EmbeddedFile) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type(path))
        .body(Body::from(file.data.into_owned()))
        .unwrap()
}

fn not_found(message: &'static str) -> Response {
    (
        StatusCode::NOT_FOUND,
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        message,
    )
        .into_response()
}

fn is_api_path(path: &str) -> bool {
    path == "api" || path.starts_with("api/")
}

fn content_type(path: &str) -> &'static str {
    match path.rsplit_once('.').map(|(_, extension)| extension) {
        Some("css") => "text/css; charset=utf-8",
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("json" | "map") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("wasm") => "application/wasm",
        _ => "application/octet-stream",
    }
}
