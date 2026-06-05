use axum::{
    Json, Router,
    extract::{
        Path, Query, State,
        rejection::{JsonRejection, PathRejection},
    },
    http::StatusCode,
    routing::{get, patch, post},
};
use utoipa_scalar::{Scalar, Servable};

use hoarder_core::types::{JobId, RunId, SourceId};

use crate::{
    api::{
        openapi,
        types::{
            CreateJobRequest, CreateSourceRequest, ErrorListQuery, FileBrowseQuery,
            FileBrowseResponse, HealthResponse, ItemDto, ItemListQuery, ItemListResponse, JobDto,
            JobListResponse, JobRunResponse, ListResponse, RunDetailDto, RunDto, RunListResponse,
            SettingsDto, SourceDto, SourceListResponse, SourceTemplateDto,
            SourceTemplateListResponse, SourceTestResponse, SyncErrorDto, SyncErrorListResponse,
            SyncStatusSchema, UpdateJobRequest, UpdateSettingsRequest, UpdateSourceRequest,
        },
    },
    app::{job_service, run_service, settings_service, source_service},
    db::repository::RuntimeSettingsRepository,
};

use super::{
    error::{ApiError, ApiErrorBody},
    state::ApiState,
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
        .route("/api/openapi.json", get(openapi_spec))
        .route("/api/source-templates", get(list_source_templates))
        .route("/api/sources", get(list_sources).post(create_source))
        .route(
            "/api/sources/{id}",
            patch(update_source).delete(delete_source),
        )
        .route("/api/sources/{id}/test", post(test_source))
        .route("/api/jobs", get(list_jobs).post(create_job))
        .route("/api/jobs/{id}", patch(update_job))
        .route("/api/jobs/{id}/run", post(run_job))
        .route("/api/jobs/{id}/stop", post(stop_job))
        .route("/api/runs", get(list_runs))
        .route("/api/runs/{id}", get(get_run_detail))
        .route("/api/files", get(browse_files))
        .route("/api/items", get(list_items))
        .route("/api/errors", get(list_errors))
        .route("/api/settings", get(settings).patch(update_settings))
        .merge(Scalar::with_url("/api/docs", openapi::document()))
}

#[utoipa::path(
    get,
    path = "/api/health",
    operation_id = "getHealth",
    tag = "system",
    responses((status = 200, description = "Success", body = HealthResponse))
)]
pub(crate) async fn health() -> Json<HealthResponse> {
    Json(HealthResponse::ok())
}

#[utoipa::path(
    get,
    path = "/api/openapi.json",
    operation_id = "getOpenApiSpec",
    tag = "system",
    responses((status = 200, description = "OpenAPI document", body = serde_json::Value))
)]
pub(crate) async fn openapi_spec() -> Json<serde_json::Value> {
    Json(openapi::spec())
}

#[utoipa::path(
    get,
    path = "/api/source-templates",
    operation_id = "listSourceTemplates",
    tag = "sources",
    responses((status = 200, description = "Success", body = SourceTemplateListResponse))
)]
pub(crate) async fn list_source_templates() -> Json<ListResponse<SourceTemplateDto>> {
    Json(ListResponse::new(source_service::source_templates()))
}

#[utoipa::path(
    get,
    path = "/api/sources",
    operation_id = "listSources",
    tag = "sources",
    responses(
        (status = 200, description = "Success", body = SourceListResponse),
        (status = 500, description = "Error", body = ApiErrorBody)
    )
)]
pub(crate) async fn list_sources(
    State(state): State<ApiState>,
) -> Result<Json<ListResponse<SourceDto>>, ApiError> {
    Ok(Json(ListResponse::new(
        source_service::list_sources(state.repository()).await?,
    )))
}

#[utoipa::path(
    post,
    path = "/api/sources",
    operation_id = "createSource",
    tag = "sources",
    request_body = CreateSourceRequest,
    responses(
        (status = 201, description = "Success", body = SourceDto),
        (status = 400, description = "Error", body = ApiErrorBody),
        (status = 500, description = "Error", body = ApiErrorBody)
    )
)]
pub(crate) async fn create_source(
    State(state): State<ApiState>,
    payload: Result<Json<CreateSourceRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<SourceDto>), ApiError> {
    let Json(request) = payload?;
    let source = source_service::create_source(state.repository(), request).await?;

    Ok((StatusCode::CREATED, Json(source)))
}

#[utoipa::path(
    patch,
    path = "/api/sources/{id}",
    operation_id = "updateSource",
    tag = "sources",
    request_body = UpdateSourceRequest,
    params(("id" = i64, Path, description = "Source identifier", minimum = 1)),
    responses(
        (status = 200, description = "Success", body = SourceDto),
        (status = 400, description = "Error", body = ApiErrorBody),
        (status = 404, description = "Error", body = ApiErrorBody),
        (status = 500, description = "Error", body = ApiErrorBody)
    )
)]
pub(crate) async fn update_source(
    State(state): State<ApiState>,
    path: Result<Path<SourceId>, PathRejection>,
    payload: Result<Json<UpdateSourceRequest>, JsonRejection>,
) -> Result<Json<SourceDto>, ApiError> {
    let Path(source_id) = path?;
    let Json(request) = payload?;

    Ok(Json(
        source_service::update_source(state.repository(), source_id, request).await?,
    ))
}

#[utoipa::path(
    delete,
    path = "/api/sources/{id}",
    operation_id = "deleteSource",
    tag = "sources",
    params(("id" = i64, Path, description = "Source identifier", minimum = 1)),
    responses(
        (status = 204, description = "Deleted"),
        (status = 404, description = "Error", body = ApiErrorBody),
        (status = 409, description = "Error", body = ApiErrorBody),
        (status = 500, description = "Error", body = ApiErrorBody)
    )
)]
pub(crate) async fn delete_source(
    State(state): State<ApiState>,
    path: Result<Path<SourceId>, PathRejection>,
) -> Result<StatusCode, ApiError> {
    let Path(source_id) = path?;
    source_service::delete_source(state.repository(), source_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/sources/{id}/test",
    operation_id = "testSource",
    tag = "sources",
    params(("id" = i64, Path, description = "Source identifier", minimum = 1)),
    responses(
        (status = 200, description = "Success", body = SourceTestResponse),
        (status = 400, description = "Error", body = ApiErrorBody),
        (status = 404, description = "Error", body = ApiErrorBody),
        (status = 502, description = "Error", body = ApiErrorBody)
    )
)]
pub(crate) async fn test_source(
    State(state): State<ApiState>,
    path: Result<Path<SourceId>, PathRejection>,
) -> Result<Json<SourceTestResponse>, ApiError> {
    let Path(source_id) = path?;

    Ok(Json(
        source_service::test_source(state.repository(), source_id).await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/jobs",
    operation_id = "listJobs",
    tag = "jobs",
    responses(
        (status = 200, description = "Success", body = JobListResponse),
        (status = 500, description = "Error", body = ApiErrorBody)
    )
)]
pub(crate) async fn list_jobs(
    State(state): State<ApiState>,
) -> Result<Json<ListResponse<JobDto>>, ApiError> {
    Ok(Json(ListResponse::new(
        job_service::list_jobs(state.repository()).await?,
    )))
}

#[utoipa::path(
    post,
    path = "/api/jobs",
    operation_id = "createJob",
    tag = "jobs",
    request_body = CreateJobRequest,
    responses(
        (status = 201, description = "Success", body = JobDto),
        (status = 400, description = "Error", body = ApiErrorBody),
        (status = 422, description = "Error", body = ApiErrorBody),
        (status = 500, description = "Error", body = ApiErrorBody)
    )
)]
pub(crate) async fn create_job(
    State(state): State<ApiState>,
    payload: Result<Json<CreateJobRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<JobDto>), ApiError> {
    let Json(request) = payload?;
    let job = job_service::create_job(state.repository(), request).await?;

    Ok((StatusCode::CREATED, Json(job)))
}

#[utoipa::path(
    patch,
    path = "/api/jobs/{id}",
    operation_id = "updateJob",
    tag = "jobs",
    request_body = UpdateJobRequest,
    params(("id" = i64, Path, description = "Job identifier", minimum = 1)),
    responses(
        (status = 200, description = "Success", body = JobDto),
        (status = 400, description = "Error", body = ApiErrorBody),
        (status = 404, description = "Error", body = ApiErrorBody),
        (status = 409, description = "Error", body = ApiErrorBody),
        (status = 422, description = "Error", body = ApiErrorBody),
        (status = 500, description = "Error", body = ApiErrorBody)
    )
)]
pub(crate) async fn update_job(
    State(state): State<ApiState>,
    path: Result<Path<JobId>, PathRejection>,
    payload: Result<Json<UpdateJobRequest>, JsonRejection>,
) -> Result<Json<JobDto>, ApiError> {
    let Path(job_id) = path?;
    let Json(request) = payload?;

    Ok(Json(
        job_service::update_job(state.repository(), job_id, request).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/api/jobs/{id}/run",
    operation_id = "runJob",
    tag = "jobs",
    params(("id" = i64, Path, description = "Job identifier", minimum = 1)),
    responses(
        (status = 200, description = "Success", body = JobRunResponse),
        (status = 404, description = "Error", body = ApiErrorBody),
        (status = 409, description = "Error", body = ApiErrorBody),
        (status = 422, description = "Error", body = ApiErrorBody),
        (status = 500, description = "Error", body = ApiErrorBody)
    )
)]
pub(crate) async fn run_job(
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
            Some(state.run_registry()),
            state.vault_path(),
            job_id,
            settings.file_concurrency,
        )
        .await?,
    ))
}

#[utoipa::path(
    post,
    path = "/api/jobs/{id}/stop",
    operation_id = "stopJob",
    tag = "jobs",
    params(("id" = i64, Path, description = "Job identifier", minimum = 1)),
    responses(
        (status = 202, description = "Stop requested"),
        (status = 404, description = "Error", body = ApiErrorBody),
        (status = 409, description = "Error", body = ApiErrorBody),
        (status = 500, description = "Error", body = ApiErrorBody)
    )
)]
pub(crate) async fn stop_job(
    State(state): State<ApiState>,
    path: Result<Path<JobId>, PathRejection>,
) -> Result<StatusCode, ApiError> {
    let Path(job_id) = path?;
    job_service::stop_job(state.repository(), state.run_registry().as_ref(), job_id).await?;

    Ok(StatusCode::ACCEPTED)
}

#[utoipa::path(
    get,
    path = "/api/runs",
    operation_id = "listRuns",
    tag = "runs",
    responses(
        (status = 200, description = "Success", body = RunListResponse),
        (status = 500, description = "Error", body = ApiErrorBody)
    )
)]
pub(crate) async fn list_runs(
    State(state): State<ApiState>,
) -> Result<Json<ListResponse<RunDto>>, ApiError> {
    Ok(Json(ListResponse::new(
        run_service::list_runs(state.repository()).await?,
    )))
}

#[utoipa::path(
    get,
    path = "/api/runs/{id}",
    operation_id = "getRunDetail",
    tag = "runs",
    params(("id" = i64, Path, description = "Run identifier", minimum = 1)),
    responses(
        (status = 200, description = "Success", body = RunDetailDto),
        (status = 404, description = "Error", body = ApiErrorBody),
        (status = 500, description = "Error", body = ApiErrorBody)
    )
)]
pub(crate) async fn get_run_detail(
    State(state): State<ApiState>,
    path: Result<Path<RunId>, PathRejection>,
) -> Result<Json<RunDetailDto>, ApiError> {
    let Path(run_id) = path?;

    Ok(Json(
        run_service::get_run_detail(state.repository(), run_id).await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/items",
    operation_id = "listItems",
    tag = "items",
    params(
        ("sourceId" = Option<i64>, Query, description = "Filter by source identifier", minimum = 1),
        ("status" = Option<SyncStatusSchema>, Query, description = "Filter by item sync status"),
        ("runId" = Option<i64>, Query, description = "Filter by run identifier", minimum = 1)
    ),
    responses(
        (status = 200, description = "Success", body = ItemListResponse),
        (status = 400, description = "Error", body = ApiErrorBody),
        (status = 500, description = "Error", body = ApiErrorBody)
    )
)]
pub(crate) async fn list_items(
    State(state): State<ApiState>,
    Query(query): Query<ItemListQuery>,
) -> Result<Json<ListResponse<ItemDto>>, ApiError> {
    Ok(Json(ListResponse::new(
        run_service::list_items(state.repository(), query).await?,
    )))
}

#[utoipa::path(
    get,
    path = "/api/files",
    operation_id = "browseFiles",
    tag = "items",
    params(
        ("sourceId" = i64, Query, description = "Source identifier", minimum = 1),
        ("path" = Option<String>, Query, description = "Source-relative directory path")
    ),
    responses(
        (status = 200, description = "Success", body = FileBrowseResponse),
        (status = 400, description = "Error", body = ApiErrorBody),
        (status = 404, description = "Error", body = ApiErrorBody),
        (status = 500, description = "Error", body = ApiErrorBody)
    )
)]
pub(crate) async fn browse_files(
    State(state): State<ApiState>,
    Query(query): Query<FileBrowseQuery>,
) -> Result<Json<FileBrowseResponse>, ApiError> {
    Ok(Json(
        run_service::browse_files(state.repository(), query).await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/errors",
    operation_id = "listErrors",
    tag = "errors",
    params(
        ("sourceId" = Option<i64>, Query, description = "Filter by source identifier", minimum = 1),
        ("runId" = Option<i64>, Query, description = "Filter by run identifier", minimum = 1)
    ),
    responses(
        (status = 200, description = "Success", body = SyncErrorListResponse),
        (status = 400, description = "Error", body = ApiErrorBody),
        (status = 500, description = "Error", body = ApiErrorBody)
    )
)]
pub(crate) async fn list_errors(
    State(state): State<ApiState>,
    Query(query): Query<ErrorListQuery>,
) -> Result<Json<ListResponse<SyncErrorDto>>, ApiError> {
    Ok(Json(ListResponse::new(
        run_service::list_errors(state.repository(), query).await?,
    )))
}

#[utoipa::path(
    get,
    path = "/api/settings",
    operation_id = "getSettings",
    tag = "settings",
    responses(
        (status = 200, description = "Success", body = SettingsDto),
        (status = 500, description = "Error", body = ApiErrorBody)
    )
)]
pub(crate) async fn settings(State(state): State<ApiState>) -> Result<Json<SettingsDto>, ApiError> {
    Ok(Json(
        settings_service::get_settings(state.repository(), state.config()).await?,
    ))
}

#[utoipa::path(
    patch,
    path = "/api/settings",
    operation_id = "updateSettings",
    tag = "settings",
    request_body = UpdateSettingsRequest,
    responses(
        (status = 200, description = "Success", body = SettingsDto),
        (status = 400, description = "Error", body = ApiErrorBody),
        (status = 500, description = "Error", body = ApiErrorBody)
    )
)]
pub(crate) async fn update_settings(
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
