use axum::{
    Json, Router,
    extract::{
        Path, Query, State,
        rejection::{JsonRejection, PathRejection},
    },
    http::StatusCode,
    routing::{get, post},
};

use crate::{
    api::{
        openapi,
        types::{
            CreateJobRequest, CreateSourceRequest, ErrorListQuery, HealthResponse, ItemDto,
            ItemListQuery, JobDto, JobRunResponse, ListResponse, RunDetailDto, RunDto, SettingsDto,
            SourceDto, SourceTestResponse, SyncErrorDto, UpdateSettingsRequest,
        },
    },
    app::{job_service, run_service, settings_service, source_service},
    core::types::{JobId, RunId, SourceId},
    db::repository::RuntimeSettingsRepository,
};

use super::{error::ApiError, state::ApiState};

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
        .route("/api/openapi.json", get(openapi_spec))
        .route("/api/sources", get(list_sources).post(create_source))
        .route("/api/sources/{id}/test", post(test_source))
        .route("/api/jobs", get(list_jobs).post(create_job))
        .route("/api/jobs/{id}/run", post(run_job))
        .route("/api/runs", get(list_runs))
        .route("/api/runs/{id}", get(get_run_detail))
        .route("/api/items", get(list_items))
        .route("/api/errors", get(list_errors))
        .route("/api/settings", get(settings).patch(update_settings))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse::ok())
}

async fn openapi_spec() -> Json<serde_json::Value> {
    Json(openapi::spec())
}

async fn list_sources(
    State(state): State<ApiState>,
) -> Result<Json<ListResponse<SourceDto>>, ApiError> {
    Ok(Json(ListResponse::new(
        source_service::list_sources(state.repository()).await?,
    )))
}

async fn create_source(
    State(state): State<ApiState>,
    payload: Result<Json<CreateSourceRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<SourceDto>), ApiError> {
    let Json(request) = payload?;
    let source = source_service::create_source(state.repository(), request).await?;

    Ok((StatusCode::CREATED, Json(source)))
}

async fn test_source(
    State(state): State<ApiState>,
    path: Result<Path<SourceId>, PathRejection>,
) -> Result<Json<SourceTestResponse>, ApiError> {
    let Path(source_id) = path?;

    Ok(Json(
        source_service::test_source(state.repository(), source_id).await?,
    ))
}

async fn list_jobs(State(state): State<ApiState>) -> Result<Json<ListResponse<JobDto>>, ApiError> {
    Ok(Json(ListResponse::new(
        job_service::list_jobs(state.repository()).await?,
    )))
}

async fn create_job(
    State(state): State<ApiState>,
    payload: Result<Json<CreateJobRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<JobDto>), ApiError> {
    let Json(request) = payload?;
    let job = job_service::create_job(state.repository(), request).await?;

    Ok((StatusCode::CREATED, Json(job)))
}

async fn run_job(
    State(state): State<ApiState>,
    path: Result<Path<JobId>, PathRejection>,
) -> Result<Json<JobRunResponse>, ApiError> {
    let Path(job_id) = path?;
    let settings = state
        .repository()
        .load_runtime_settings(state.config())
        .await?;

    Ok(Json(
        job_service::run_job(
            std::sync::Arc::clone(state.repository()),
            state.vault_path(),
            job_id,
            settings.file_concurrency,
        )
        .await?,
    ))
}

async fn list_runs(State(state): State<ApiState>) -> Result<Json<ListResponse<RunDto>>, ApiError> {
    Ok(Json(ListResponse::new(
        run_service::list_runs(state.repository()).await?,
    )))
}

async fn get_run_detail(
    State(state): State<ApiState>,
    path: Result<Path<RunId>, PathRejection>,
) -> Result<Json<RunDetailDto>, ApiError> {
    let Path(run_id) = path?;

    Ok(Json(
        run_service::get_run_detail(state.repository(), run_id).await?,
    ))
}

async fn list_items(
    State(state): State<ApiState>,
    Query(query): Query<ItemListQuery>,
) -> Result<Json<ListResponse<ItemDto>>, ApiError> {
    Ok(Json(ListResponse::new(
        run_service::list_items(state.repository(), query).await?,
    )))
}

async fn list_errors(
    State(state): State<ApiState>,
    Query(query): Query<ErrorListQuery>,
) -> Result<Json<ListResponse<SyncErrorDto>>, ApiError> {
    Ok(Json(ListResponse::new(
        run_service::list_errors(state.repository(), query).await?,
    )))
}

async fn settings(State(state): State<ApiState>) -> Result<Json<SettingsDto>, ApiError> {
    Ok(Json(
        settings_service::get_settings(state.repository(), state.config()).await?,
    ))
}

async fn update_settings(
    State(state): State<ApiState>,
    payload: Result<Json<UpdateSettingsRequest>, JsonRejection>,
) -> Result<Json<SettingsDto>, ApiError> {
    let Json(request) = payload?;

    Ok(Json(
        settings_service::update_settings(state.repository(), state.config(), request).await?,
    ))
}

async fn api_not_found() -> ApiError {
    ApiError::not_found("API route not found")
}
