use serde_json::Value;
use utoipa::OpenApi;

use crate::api::{
    error::{ApiErrorBody, ApiErrorDetail},
    types::{
        ConnectorConfigSchema, ConnectorKindSchema, CreateJobRequest, CreateSourceRequest,
        FileBrowseResponse, FileEntryDto, FileEntryKind, HealthResponse, ItemDto, ItemListResponse,
        ItemTypeSchema, JobDto, JobListResponse, JobRunResponse, JobScheduleDto, JobStatusSchema,
        ReadOnlySettingsDto, RedactedConnectorConfig, RunCountsDto, RunDetailDto, RunDto,
        RunListResponse, RunStatusSchema, SettingsDto, SourceDto, SourceHealth, SourceListResponse,
        SourceTemplateDto, SourceTemplateListResponse, SourceTemplateOptionDto, SourceTestResponse,
        SyncErrorDto, SyncErrorListResponse, SyncStatusSchema, UpdateJobRequest,
        UpdateSettingsRequest, UpdateSourceRequest,
    },
};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::api::routes::health,
        crate::api::routes::openapi_spec,
        crate::api::routes::list_source_templates,
        crate::api::routes::list_sources,
        crate::api::routes::create_source,
        crate::api::routes::update_source,
        crate::api::routes::delete_source,
        crate::api::routes::test_source,
        crate::api::routes::list_jobs,
        crate::api::routes::create_job,
        crate::api::routes::update_job,
        crate::api::routes::run_job,
        crate::api::routes::stop_job,
        crate::api::routes::list_runs,
        crate::api::routes::get_run_detail,
        crate::api::routes::browse_files,
        crate::api::routes::list_items,
        crate::api::routes::list_errors,
        crate::api::routes::settings,
        crate::api::routes::update_settings,
    ),
    components(schemas(
        ApiErrorBody,
        ApiErrorDetail,
        ConnectorConfigSchema,
        ConnectorKindSchema,
        CreateJobRequest,
        CreateSourceRequest,
        FileBrowseResponse,
        FileEntryDto,
        FileEntryKind,
        HealthResponse,
        ItemDto,
        ItemListResponse,
        ItemTypeSchema,
        JobDto,
        JobListResponse,
        JobRunResponse,
        JobScheduleDto,
        JobStatusSchema,
        ReadOnlySettingsDto,
        RedactedConnectorConfig,
        RunCountsDto,
        RunDetailDto,
        RunDto,
        RunListResponse,
        RunStatusSchema,
        SettingsDto,
        SourceDto,
        SourceHealth,
        SourceListResponse,
        SourceTemplateDto,
        SourceTemplateListResponse,
        SourceTemplateOptionDto,
        SourceTestResponse,
        SyncErrorDto,
        SyncErrorListResponse,
        SyncStatusSchema,
        UpdateJobRequest,
        UpdateSettingsRequest,
        UpdateSourceRequest,
    )),
    info(
        title = "Hoarder Local API",
        description = "Local-first one-way data aggregation control plane API."
    ),
    servers((url = "/"))
)]
struct ApiDoc;

/// Returns the generated machine-readable `OpenAPI` contract for the local Hoarder API.
#[must_use]
pub fn document() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}

/// Returns the generated `OpenAPI` contract as JSON for the HTTP endpoint.
///
/// # Panics
///
/// Panics if the generated `utoipa` document cannot be serialized to JSON.
#[must_use]
pub fn spec() -> Value {
    let mut spec =
        serde_json::to_value(document()).expect("generated OpenAPI document serializes to JSON");
    inline_local_id_refs(&mut spec);
    inline_simple_enum_refs(&mut spec);
    normalize_nullable_local_ids(&mut spec);

    spec
}

fn inline_simple_enum_refs(spec: &mut Value) {
    const ENUM_SCHEMAS: &[&str] = &[
        "ConnectorKindSchema",
        "FileEntryKind",
        "ItemTypeSchema",
        "JobStatusSchema",
        "RunStatusSchema",
        "SourceHealth",
        "SyncStatusSchema",
    ];

    let Some(schemas) = spec
        .pointer("/components/schemas")
        .and_then(Value::as_object)
        .cloned()
    else {
        return;
    };

    inline_named_component_refs(spec, &schemas, ENUM_SCHEMAS);
}

fn inline_named_component_refs(
    value: &mut Value,
    schemas: &serde_json::Map<String, Value>,
    names: &[&str],
) {
    match value {
        Value::Array(values) => {
            for value in values {
                inline_named_component_refs(value, schemas, names);
            }
        }
        Value::Object(object) => {
            if object.len() == 1
                && let Some(name) = object
                    .get("$ref")
                    .and_then(Value::as_str)
                    .and_then(component_ref_name)
                && names.contains(&name)
                && let Some(schema) = schemas.get(name)
            {
                *value = schema.clone();
                return;
            }

            for value in object.values_mut() {
                inline_named_component_refs(value, schemas, names);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn inline_local_id_refs(value: &mut Value) {
    match value {
        Value::Array(values) => {
            for value in values {
                inline_local_id_refs(value);
            }
        }
        Value::Object(object) => {
            if object.len() == 1
                && object
                    .get("$ref")
                    .and_then(Value::as_str)
                    .is_some_and(is_local_id_ref)
            {
                *value = local_id_schema();
                return;
            }

            for value in object.values_mut() {
                inline_local_id_refs(value);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn normalize_nullable_local_ids(value: &mut Value) {
    match value {
        Value::Array(values) => {
            for value in values {
                normalize_nullable_local_ids(value);
            }
        }
        Value::Object(object) => {
            for key in ["anyOf", "oneOf"] {
                if object
                    .get(key)
                    .and_then(Value::as_array)
                    .is_some_and(|values| is_nullable_local_id_schema(values))
                {
                    *value = serde_json::json!({
                        "type": ["integer", "null"],
                        "minimum": 1
                    });
                    return;
                }
            }

            for value in object.values_mut() {
                normalize_nullable_local_ids(value);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn is_nullable_local_id_schema(values: &[Value]) -> bool {
    values.len() == 2 && values.iter().any(is_local_id_schema) && values.iter().any(is_null_schema)
}

fn is_local_id_schema(value: &Value) -> bool {
    value.get("type") == Some(&Value::String("integer".to_owned()))
        && value.get("minimum").and_then(Value::as_i64) == Some(1)
}

fn is_null_schema(value: &Value) -> bool {
    value.get("type") == Some(&Value::String("null".to_owned()))
}

fn is_local_id_ref(reference: &str) -> bool {
    matches!(
        reference,
        "#/components/schemas/SourceId"
            | "#/components/schemas/JobId"
            | "#/components/schemas/RunId"
            | "#/components/schemas/ItemId"
    )
}

fn component_ref_name(reference: &str) -> Option<&str> {
    reference.strip_prefix("#/components/schemas/")
}

fn local_id_schema() -> Value {
    serde_json::json!({
        "type": "integer",
        "minimum": 1
    })
}
