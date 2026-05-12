use axum::{
    Json, Router,
    extract::{
        Path, State,
        rejection::{JsonRejection, PathRejection},
    },
    http::StatusCode,
    routing::{get, post},
};

use crate::core::types::JobId;

use super::{
    error::ApiError,
    state::ApiState,
    types::{
        CreateSourceRequest, HealthResponse, ItemDto, JobDto, JobRunResponse, ListResponse, RunDto,
        SettingsDto, SourceDto, SyncErrorDto,
    },
};

pub fn router(state: ApiState) -> Router {
    api_routes_without_state()
        .fallback(api_not_found)
        .with_state(state)
}

pub fn router_without_fallback(state: ApiState) -> Router {
    api_routes_without_state().with_state(state)
}

fn api_routes_without_state() -> Router<ApiState> {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/sources", get(list_sources).post(create_source))
        .route("/api/jobs", get(list_jobs))
        .route("/api/jobs/{id}/run", post(run_job))
        .route("/api/runs", get(list_runs))
        .route("/api/items", get(list_items))
        .route("/api/errors", get(list_errors))
        .route("/api/settings", get(settings))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse::ok())
}

async fn list_sources(
    State(state): State<ApiState>,
) -> Result<Json<ListResponse<SourceDto>>, ApiError> {
    let sources = state.repository().list_sources().await?;

    Ok(Json(ListResponse::new(sources)))
}

async fn create_source(
    State(state): State<ApiState>,
    payload: Result<Json<CreateSourceRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<SourceDto>), ApiError> {
    let Json(request) = payload?;
    let source = state.repository().create_source(request).await?;

    Ok((StatusCode::CREATED, Json(source)))
}

async fn list_jobs(State(state): State<ApiState>) -> Result<Json<ListResponse<JobDto>>, ApiError> {
    let jobs = state.repository().list_jobs().await?;

    Ok(Json(ListResponse::new(jobs)))
}

async fn run_job(
    State(state): State<ApiState>,
    path: Result<Path<JobId>, PathRejection>,
) -> Result<Json<JobRunResponse>, ApiError> {
    let Path(job_id) = path?;
    let run = state.sync_service().run_job(job_id).await?;

    Ok(Json(run))
}

async fn list_runs(State(state): State<ApiState>) -> Result<Json<ListResponse<RunDto>>, ApiError> {
    let runs = state.repository().list_runs().await?;

    Ok(Json(ListResponse::new(runs)))
}

async fn list_items(
    State(state): State<ApiState>,
) -> Result<Json<ListResponse<ItemDto>>, ApiError> {
    let items = state.repository().list_items().await?;

    Ok(Json(ListResponse::new(items)))
}

async fn list_errors(
    State(state): State<ApiState>,
) -> Result<Json<ListResponse<SyncErrorDto>>, ApiError> {
    let errors = state.repository().list_errors().await?;

    Ok(Json(ListResponse::new(errors)))
}

async fn settings(State(state): State<ApiState>) -> Result<Json<SettingsDto>, ApiError> {
    let settings = state.repository().settings().await?;

    Ok(Json(settings))
}

async fn api_not_found() -> ApiError {
    ApiError::not_found("API route not found")
}
