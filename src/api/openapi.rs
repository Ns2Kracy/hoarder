use serde_json::{Value, json};

/// Returns the machine-readable `OpenAPI` contract for the local Hoarder API.
#[must_use]
pub fn spec() -> Value {
    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Hoarder Local API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Local-first one-way data aggregation control plane API."
        },
        "servers": [{"url": "/"}],
        "paths": paths(),
        "components": {
            "schemas": schemas()
        }
    })
}

#[must_use]
fn paths() -> Value {
    json!({
        "/api/health": health_path(),
        "/api/openapi.json": openapi_path(),
        "/api/sources": sources_path(),
        "/api/sources/{id}/test": source_test_path(),
        "/api/jobs": jobs_path(),
        "/api/jobs/{id}/run": job_run_path(),
        "/api/runs": runs_path(),
        "/api/runs/{id}": run_detail_path(),
        "/api/items": items_path(),
        "/api/errors": errors_path(),
        "/api/settings": settings_path()
    })
}

#[must_use]
fn health_path() -> Value {
    json!({
        "get": {
            "tags": ["system"],
            "operationId": "getHealth",
            "responses": {
                "200": json_response("HealthResponse")
            }
        }
    })
}

#[must_use]
fn openapi_path() -> Value {
    json!({
        "get": {
            "tags": ["system"],
            "operationId": "getOpenApiSpec",
            "responses": {
                "200": {
                    "description": "OpenAPI document",
                    "content": {
                        "application/json": {
                            "schema": {
                                "type": "object"
                            }
                        }
                    }
                }
            }
        }
    })
}

#[must_use]
fn sources_path() -> Value {
    json!({
        "get": {
            "tags": ["sources"],
            "operationId": "listSources",
            "responses": {
                "200": list_response("SourceDto"),
                "500": error_response()
            }
        },
        "post": {
            "tags": ["sources"],
            "operationId": "createSource",
            "requestBody": request_body("CreateSourceRequest"),
            "responses": {
                "201": json_response("SourceDto"),
                "400": error_response(),
                "500": error_response()
            }
        }
    })
}

#[must_use]
fn source_test_path() -> Value {
    json!({
        "post": {
            "tags": ["sources"],
            "operationId": "testSource",
            "parameters": [path_uuid_parameter("id", "Source identifier")],
            "responses": {
                "200": json_response("SourceTestResponse"),
                "400": error_response(),
                "404": error_response(),
                "502": error_response()
            }
        }
    })
}

#[must_use]
fn jobs_path() -> Value {
    json!({
        "get": {
            "tags": ["jobs"],
            "operationId": "listJobs",
            "responses": {
                "200": list_response("JobDto"),
                "500": error_response()
            }
        },
        "post": {
            "tags": ["jobs"],
            "operationId": "createJob",
            "requestBody": request_body("CreateJobRequest"),
            "responses": {
                "201": json_response("JobDto"),
                "400": error_response(),
                "422": error_response(),
                "500": error_response()
            }
        }
    })
}

#[must_use]
fn job_run_path() -> Value {
    json!({
        "post": {
            "tags": ["jobs"],
            "operationId": "runJob",
            "parameters": [path_uuid_parameter("id", "Job identifier")],
            "responses": {
                "200": json_response("JobRunResponse"),
                "404": error_response(),
                "409": error_response(),
                "422": error_response(),
                "500": error_response()
            }
        }
    })
}

#[must_use]
fn runs_path() -> Value {
    json!({
        "get": {
            "tags": ["runs"],
            "operationId": "listRuns",
            "responses": {
                "200": list_response("RunDto"),
                "500": error_response()
            }
        }
    })
}

#[must_use]
fn run_detail_path() -> Value {
    json!({
        "get": {
            "tags": ["runs"],
            "operationId": "getRunDetail",
            "parameters": [path_uuid_parameter("id", "Run identifier")],
            "responses": {
                "200": json_response("RunDetailDto"),
                "404": error_response(),
                "500": error_response()
            }
        }
    })
}

#[must_use]
fn items_path() -> Value {
    json!({
        "get": {
            "tags": ["items"],
            "operationId": "listItems",
            "parameters": [
                query_uuid_parameter("sourceId", "Filter by source identifier"),
                query_enum_parameter("status", "SyncStatus", "Filter by item sync status"),
                query_uuid_parameter("runId", "Filter by run identifier")
            ],
            "responses": {
                "200": list_response("ItemDto"),
                "400": error_response(),
                "500": error_response()
            }
        }
    })
}

#[must_use]
fn errors_path() -> Value {
    json!({
        "get": {
            "tags": ["errors"],
            "operationId": "listErrors",
            "parameters": [
                query_uuid_parameter("sourceId", "Filter by source identifier"),
                query_uuid_parameter("runId", "Filter by run identifier")
            ],
            "responses": {
                "200": list_response("SyncErrorDto"),
                "400": error_response(),
                "500": error_response()
            }
        }
    })
}

#[must_use]
fn settings_path() -> Value {
    json!({
        "get": {
            "tags": ["settings"],
            "operationId": "getSettings",
            "responses": {
                "200": json_response("SettingsDto"),
                "500": error_response()
            }
        },
        "patch": {
            "tags": ["settings"],
            "operationId": "updateSettings",
            "requestBody": request_body("UpdateSettingsRequest"),
            "responses": {
                "200": json_response("SettingsDto"),
                "400": error_response(),
                "500": error_response()
            }
        }
    })
}

#[must_use]
fn schemas() -> Value {
    json!({
        "ApiErrorBody": api_error_body_schema(),
        "CreateJobRequest": create_job_request_schema(),
        "CreateSourceRequest": create_source_request_schema(),
        "HealthResponse": health_response_schema(),
        "ItemDto": item_schema(),
        "JobDto": job_schema(),
        "JobRunResponse": job_run_response_schema(),
        "JobScheduleDto": job_schedule_schema(),
        "ReadOnlySettingsDto": read_only_settings_schema(),
        "RedactedConnectorConfig": connector_config_schema(),
        "RunCountsDto": run_counts_schema(),
        "RunDetailDto": run_detail_schema(),
        "RunDto": run_schema(),
        "SettingsDto": settings_schema(),
        "SourceDto": source_schema(),
        "SourceTestResponse": source_test_response_schema(),
        "SyncErrorDto": sync_error_schema(),
        "UpdateSettingsRequest": update_settings_request_schema()
    })
}

#[must_use]
fn api_error_body_schema() -> Value {
    json!({
        "type": "object",
        "required": ["error"],
        "properties": {
            "error": {
                "type": "object",
                "required": ["code", "message"],
                "properties": {
                    "code": {"type": "string"},
                    "message": {"type": "string"}
                }
            }
        }
    })
}

#[must_use]
fn create_job_request_schema() -> Value {
    json!({
        "type": "object",
        "required": ["sourceId", "name", "schedule"],
        "properties": {
            "sourceId": uuid_schema(),
            "name": {"type": "string"},
            "enabled": {"type": "boolean", "default": true},
            "schedule": ref_schema("JobScheduleDto")
        }
    })
}

#[must_use]
fn create_source_request_schema() -> Value {
    json!({
        "type": "object",
        "required": ["name", "config"],
        "properties": {
            "name": {"type": "string"},
            "config": ref_schema("RedactedConnectorConfig"),
            "enabled": {"type": "boolean", "default": true}
        }
    })
}

#[must_use]
fn health_response_schema() -> Value {
    json!({
        "type": "object",
        "required": ["status"],
        "properties": {
            "status": {"type": "string", "enum": ["ok"]}
        }
    })
}

#[must_use]
fn item_schema() -> Value {
    json!({
        "type": "object",
        "required": ["id", "sourceId", "sourcePath", "itemType", "status"],
        "properties": {
            "id": uuid_schema(),
            "sourceId": uuid_schema(),
            "sourcePath": {"type": "string"},
            "itemType": {"type": "string", "enum": ["file", "directory", "virtual_document"]},
            "status": sync_status_schema(),
            "size": nullable_integer_schema(),
            "etag": nullable_string_schema(),
            "modifiedAt": nullable_datetime_schema(),
            "contentHash": nullable_string_schema(),
            "metadataJson": {"type": ["object", "array", "string", "number", "boolean", "null"]}
        }
    })
}

#[must_use]
fn job_schema() -> Value {
    json!({
        "type": "object",
        "required": ["id", "sourceId", "name", "enabled", "schedule", "status"],
        "properties": {
            "id": uuid_schema(),
            "sourceId": uuid_schema(),
            "name": {"type": "string"},
            "enabled": {"type": "boolean"},
            "schedule": ref_schema("JobScheduleDto"),
            "status": job_status_schema(),
            "lastRunAt": nullable_datetime_schema(),
            "lastRunStatus": nullable_run_status_schema(),
            "lastRunId": nullable_uuid_schema(),
            "nextRunAt": nullable_datetime_schema()
        }
    })
}

#[must_use]
fn job_run_response_schema() -> Value {
    json!({
        "type": "object",
        "required": ["runId", "status"],
        "properties": {
            "runId": uuid_schema(),
            "status": sync_status_schema()
        }
    })
}

#[must_use]
fn job_schedule_schema() -> Value {
    json!({
        "oneOf": [
            {
                "type": "object",
                "required": ["kind"],
                "properties": {
                    "kind": {"type": "string", "enum": ["manual"]}
                }
            },
            {
                "type": "object",
                "required": ["kind", "intervalSeconds"],
                "properties": {
                    "kind": {"type": "string", "enum": ["interval"]},
                    "intervalSeconds": {"type": "integer", "minimum": 1}
                }
            }
        ]
    })
}

#[must_use]
fn read_only_settings_schema() -> Value {
    json!({
        "type": "object",
        "required": ["databasePath", "vaultPath", "listenAddr"],
        "properties": {
            "databasePath": {"type": "boolean"},
            "vaultPath": {"type": "boolean"},
            "listenAddr": {"type": "boolean"}
        }
    })
}

#[must_use]
fn connector_config_schema() -> Value {
    json!({
        "type": "object",
        "required": ["kind", "service", "options"],
        "properties": {
            "kind": {"type": "string", "enum": ["opendal"]},
            "service": {"type": "string", "enum": ["fs", "webdav", "sftp", "s3"]},
            "options": {
                "type": "object",
                "additionalProperties": {"type": "string"}
            }
        }
    })
}

#[must_use]
fn run_counts_schema() -> Value {
    json!({
        "type": "object",
        "required": ["processed", "synced", "skipped", "failed", "deleted"],
        "properties": {
            "processed": {"type": "integer", "minimum": 0},
            "synced": {"type": "integer", "minimum": 0},
            "skipped": {"type": "integer", "minimum": 0},
            "failed": {"type": "integer", "minimum": 0},
            "deleted": {"type": "integer", "minimum": 0}
        }
    })
}

#[must_use]
fn run_detail_schema() -> Value {
    json!({
        "type": "object",
        "required": ["id", "jobId", "sourceId", "sourceName", "jobName", "status", "counts", "errors"],
        "properties": {
            "id": uuid_schema(),
            "jobId": uuid_schema(),
            "sourceId": uuid_schema(),
            "sourceName": {"type": "string"},
            "jobName": {"type": "string"},
            "status": run_status_schema(),
            "startedAt": nullable_datetime_schema(),
            "finishedAt": nullable_datetime_schema(),
            "durationMs": nullable_integer_schema(),
            "counts": ref_schema("RunCountsDto"),
            "errors": array_ref_schema("SyncErrorDto")
        }
    })
}

#[must_use]
fn run_schema() -> Value {
    json!({
        "type": "object",
        "required": ["id", "jobId", "status", "processedCount", "syncedCount", "skippedCount", "failedCount"],
        "properties": {
            "id": uuid_schema(),
            "jobId": uuid_schema(),
            "status": sync_status_schema(),
            "startedAt": nullable_datetime_schema(),
            "finishedAt": nullable_datetime_schema(),
            "processedCount": {"type": "integer", "minimum": 0},
            "syncedCount": {"type": "integer", "minimum": 0},
            "skippedCount": {"type": "integer", "minimum": 0},
            "failedCount": {"type": "integer", "minimum": 0}
        }
    })
}

#[must_use]
fn settings_schema() -> Value {
    json!({
        "type": "object",
        "required": ["databasePath", "vaultPath", "listenAddr", "jobConcurrency", "fileConcurrency", "logLevel", "readOnly"],
        "properties": {
            "databasePath": {"type": "string"},
            "vaultPath": {"type": "string"},
            "listenAddr": {"type": "string"},
            "jobConcurrency": {"type": "integer", "minimum": 1},
            "fileConcurrency": {"type": "integer", "minimum": 1},
            "logLevel": log_level_schema(),
            "readOnly": ref_schema("ReadOnlySettingsDto")
        }
    })
}

#[must_use]
fn source_schema() -> Value {
    json!({
        "type": "object",
        "required": ["id", "name", "connectorKind", "config", "enabled", "health"],
        "properties": {
            "id": uuid_schema(),
            "name": {"type": "string"},
            "connectorKind": {"type": "string", "enum": ["opendal"]},
            "config": ref_schema("RedactedConnectorConfig"),
            "enabled": {"type": "boolean"},
            "health": {"type": "string", "enum": ["healthy", "warning", "failed", "untested", "disabled"]},
            "lastCheckedAt": nullable_datetime_schema()
        }
    })
}

#[must_use]
fn source_test_response_schema() -> Value {
    json!({
        "type": "object",
        "required": ["ok", "checkedAt"],
        "properties": {
            "ok": {"type": "boolean"},
            "checkedAt": datetime_schema()
        }
    })
}

#[must_use]
fn sync_error_schema() -> Value {
    json!({
        "type": "object",
        "required": ["id", "code", "message"],
        "properties": {
            "id": {"type": "string"},
            "runId": nullable_uuid_schema(),
            "sourceId": nullable_uuid_schema(),
            "sourcePath": nullable_string_schema(),
            "code": {"type": "string"},
            "message": {"type": "string"},
            "createdAt": nullable_datetime_schema()
        }
    })
}

#[must_use]
fn update_settings_request_schema() -> Value {
    json!({
        "type": "object",
        "required": ["jobConcurrency", "fileConcurrency", "logLevel"],
        "properties": {
            "jobConcurrency": {"type": "integer", "minimum": 1},
            "fileConcurrency": {"type": "integer", "minimum": 1},
            "logLevel": log_level_schema()
        }
    })
}

#[must_use]
fn request_body(schema_name: &str) -> Value {
    json!({
        "required": true,
        "content": {
            "application/json": {
                "schema": ref_schema(schema_name)
            }
        }
    })
}

#[must_use]
fn json_response(schema_name: &str) -> Value {
    json!({
        "description": "Success",
        "content": {
            "application/json": {
                "schema": ref_schema(schema_name)
            }
        }
    })
}

#[must_use]
fn list_response(item_schema_name: &str) -> Value {
    json!({
        "description": "Success",
        "content": {
            "application/json": {
                "schema": {
                    "type": "object",
                    "required": ["data"],
                    "properties": {
                        "data": array_ref_schema(item_schema_name)
                    }
                }
            }
        }
    })
}

#[must_use]
fn error_response() -> Value {
    json!({
        "description": "Error",
        "content": {
            "application/json": {
                "schema": ref_schema("ApiErrorBody")
            }
        }
    })
}

#[must_use]
fn path_uuid_parameter(name: &str, description: &str) -> Value {
    json!({
        "name": name,
        "in": "path",
        "required": true,
        "description": description,
        "schema": uuid_schema()
    })
}

#[must_use]
fn query_uuid_parameter(name: &str, description: &str) -> Value {
    json!({
        "name": name,
        "in": "query",
        "required": false,
        "description": description,
        "schema": uuid_schema()
    })
}

#[must_use]
fn query_enum_parameter(name: &str, schema_name: &str, description: &str) -> Value {
    json!({
        "name": name,
        "in": "query",
        "required": false,
        "description": description,
        "schema": ref_schema(schema_name)
    })
}

#[must_use]
fn ref_schema(schema_name: &str) -> Value {
    json!({"$ref": format!("#/components/schemas/{schema_name}")})
}

#[must_use]
fn array_ref_schema(schema_name: &str) -> Value {
    json!({
        "type": "array",
        "items": ref_schema(schema_name)
    })
}

#[must_use]
fn uuid_schema() -> Value {
    json!({"type": "string", "format": "uuid"})
}

#[must_use]
fn nullable_uuid_schema() -> Value {
    json!({"type": ["string", "null"], "format": "uuid"})
}

#[must_use]
fn datetime_schema() -> Value {
    json!({"type": "string", "format": "date-time"})
}

#[must_use]
fn nullable_datetime_schema() -> Value {
    json!({"type": ["string", "null"], "format": "date-time"})
}

#[must_use]
fn nullable_string_schema() -> Value {
    json!({"type": ["string", "null"]})
}

#[must_use]
fn nullable_integer_schema() -> Value {
    json!({"type": ["integer", "null"], "minimum": 0})
}

#[must_use]
fn job_status_schema() -> Value {
    json!({"type": "string", "enum": ["idle", "running", "paused", "failed"]})
}

#[must_use]
fn run_status_schema() -> Value {
    json!({"type": "string", "enum": ["running", "completed", "completed_with_failures", "failed", "cancelled"]})
}

#[must_use]
fn nullable_run_status_schema() -> Value {
    json!({"type": ["string", "null"], "enum": ["running", "completed", "completed_with_failures", "failed", "cancelled", null]})
}

#[must_use]
fn sync_status_schema() -> Value {
    json!({"type": "string", "enum": ["pending", "synced", "skipped", "failed", "deleted_on_source"]})
}

#[must_use]
fn log_level_schema() -> Value {
    json!({"type": "string", "enum": ["trace", "debug", "info", "warn", "error"]})
}
