use bytes::Bytes;
use chrono::{DateTime, Utc};
use futures::{FutureExt, stream};
use reqwest::{Client, Method, RequestBuilder, StatusCode};
use serde_json::{Value, json};

use crate::{
    AppError, AppResult,
    connectors::traits::{
        ByteStream, ConnectorConfig, ConnectorFuture, ScanOutcome, ScanStream, SourceConnector,
    },
    core::types::{
        ConnectorCapabilities, ConnectorKind, ItemRef, ItemSnapshot, ItemType, SourceId,
    },
};

const DEFAULT_BASE_URL: &str = "https://open.feishu.cn";
const PAGE_SIZE: u16 = 50;

#[derive(Clone, Debug)]
pub struct FeishuSourceConnector {
    source_id: SourceId,
    client: Client,
}

impl FeishuSourceConnector {
    #[must_use]
    pub fn new(source_id: SourceId) -> Self {
        Self {
            source_id,
            client: Client::new(),
        }
    }

    #[must_use]
    pub const fn source_id(&self) -> SourceId {
        self.source_id
    }
}

impl SourceConnector for FeishuSourceConnector {
    fn kind(&self) -> ConnectorKind {
        ConnectorKind::Feishu
    }

    fn validate<'a>(
        &'a self,
        config: &'a ConnectorConfig,
    ) -> ConnectorFuture<'a, ConnectorCapabilities> {
        async move {
            let config = feishu_config(config)?;
            self.tenant_access_token(&config).await?;

            Ok(capabilities())
        }
        .boxed()
    }

    fn scan<'a>(
        &'a self,
        config: &'a ConnectorConfig,
        cursor: Option<&'a str>,
    ) -> ConnectorFuture<'a, ScanOutcome> {
        async move {
            let config = feishu_config(config)?;
            let tenant_token = self.tenant_access_token(&config).await?;
            let response = self.list_files(&config, &tenant_token, cursor).await?;
            let next_cursor = feishu_next_cursor(&response)?;
            let snapshots = feishu_snapshots(self.source_id, &response)?;
            let items = Box::pin(stream::iter(snapshots.into_iter().map(Ok))) as ScanStream;

            Ok(ScanOutcome::new(items, next_cursor))
        }
        .boxed()
    }

    fn read<'a>(
        &'a self,
        config: &'a ConnectorConfig,
        item_ref: &'a ItemRef,
    ) -> ConnectorFuture<'a, ByteStream> {
        async move {
            let _config = feishu_config(config)?;
            let (file_type, token) = feishu_item_parts(&item_ref.source_path)?;
            let document = json!({
                "provider": "feishu",
                "kind": file_type,
                "token": token,
                "sourcePath": item_ref.source_path,
                "contentMode": "reference"
            });
            let bytes = serde_json::to_vec_pretty(&document).map_err(|error| {
                AppError::Connector(format!("serialize Feishu virtual document: {error}"))
            })?;
            let bytes = Bytes::from(bytes);

            Ok(Box::pin(stream::once(async move { Ok(bytes) })) as ByteStream)
        }
        .boxed()
    }
}

impl FeishuSourceConnector {
    async fn tenant_access_token(&self, config: &ResolvedFeishuConfig<'_>) -> AppResult<String> {
        let body = json!({
            "app_id": config.app_id,
            "app_secret": config.app_secret
        });
        let response = self
            .request(
                config,
                Method::POST,
                "/open-apis/auth/v3/tenant_access_token/internal",
            )
            .json(&body)
            .send_json("get Feishu tenant access token")
            .await?;
        feishu_code_ok(&response, "get Feishu tenant access token")?;

        response
            .get("tenant_access_token")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .ok_or_else(|| {
                AppError::Connector(
                    "Feishu tenant token response missing tenant_access_token".to_owned(),
                )
            })
    }

    async fn list_files(
        &self,
        config: &ResolvedFeishuConfig<'_>,
        tenant_token: &str,
        cursor: Option<&str>,
    ) -> AppResult<Value> {
        let folder_token = config.folder_token.ok_or_else(|| {
            AppError::Validation("feishu connector requires folderToken".to_owned())
        })?;
        let mut request = self
            .request(config, Method::GET, "/open-apis/drive/v1/files")
            .bearer_auth(tenant_token)
            .query(&[
                ("folder_token", folder_token.to_owned()),
                ("page_size", PAGE_SIZE.to_string()),
            ]);
        if let Some(cursor) = cursor {
            request = request.query(&[("page_token", cursor)]);
        }
        let response = request.send_json("list Feishu drive files").await?;
        feishu_code_ok(&response, "list Feishu drive files")?;

        Ok(response)
    }

    fn request(
        &self,
        config: &ResolvedFeishuConfig<'_>,
        method: Method,
        path: &str,
    ) -> RequestBuilder {
        self.client
            .request(method, format!("{}{}", config.base_url, path))
            .header("Accept", "application/json")
    }
}

#[derive(Clone, Debug)]
struct ResolvedFeishuConfig<'a> {
    app_id: &'a str,
    app_secret: &'a str,
    folder_token: Option<&'a str>,
    base_url: String,
}

fn feishu_config(config: &ConnectorConfig) -> AppResult<ResolvedFeishuConfig<'_>> {
    let ConnectorConfig::Feishu {
        app_id,
        app_secret,
        folder_token,
        base_url,
    } = config
    else {
        return Err(AppError::Validation(
            "expected feishu connector config".to_owned(),
        ));
    };
    require_non_empty("appId", app_id)?;
    require_non_empty("appSecret", app_secret)?;

    Ok(ResolvedFeishuConfig {
        app_id,
        app_secret,
        folder_token: folder_token.as_deref().filter(|value| !value.is_empty()),
        base_url: base_url
            .as_deref()
            .unwrap_or(DEFAULT_BASE_URL)
            .trim_end_matches('/')
            .to_owned(),
    })
}

const fn capabilities() -> ConnectorCapabilities {
    ConnectorCapabilities {
        supports_files: false,
        supports_directories: true,
        supports_virtual_documents: true,
        supports_incremental_scan: true,
    }
}

fn feishu_snapshots(source_id: SourceId, response: &Value) -> AppResult<Vec<ItemSnapshot>> {
    let files = response
        .get("data")
        .and_then(|data| data.get("files"))
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::Connector("Feishu response missing data.files".to_owned()))?;

    files
        .iter()
        .map(|file| feishu_snapshot(source_id, file))
        .collect()
}

fn feishu_snapshot(source_id: SourceId, file: &Value) -> AppResult<ItemSnapshot> {
    let token = string_field(file, &["token", "file_token"], "Feishu file")?;
    let file_type = string_field(file, &["type", "file_type"], "Feishu file").unwrap_or("file");
    let source_path = format!("feishu/{file_type}/{token}.json");
    let modified = string_field(
        file,
        &["modified_time", "edit_time", "updated_time"],
        "Feishu file",
    )
    .ok()
    .map(ToOwned::to_owned);
    let item_type = if file_type == "folder" {
        ItemType::Directory
    } else {
        ItemType::VirtualDocument
    };

    Ok(ItemSnapshot {
        source_id,
        source_path,
        item_type,
        size: optional_u64(file, &["size", "file_size"]),
        etag: modified.clone().or_else(|| Some(token.to_owned())),
        modified_at: modified.as_deref().and_then(parse_feishu_time),
        content_hash: None,
        metadata_json: Some(json!({
            "provider": "feishu",
            "token": token,
            "name": file.get("name").and_then(Value::as_str),
            "type": file_type,
            "url": file.get("url").and_then(Value::as_str),
            "raw": file
        })),
    })
}

fn feishu_next_cursor(response: &Value) -> AppResult<Option<String>> {
    let data = response
        .get("data")
        .ok_or_else(|| AppError::Connector("Feishu response missing data".to_owned()))?;
    let has_more = data
        .get("has_more")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !has_more {
        return Ok(None);
    }

    data.get("next_page_token")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| AppError::Connector("Feishu response missing next_page_token".to_owned()))
        .map(Some)
}

fn feishu_item_parts(source_path: &str) -> AppResult<(String, String)> {
    let value = source_path
        .strip_prefix("feishu/")
        .and_then(|value| value.strip_suffix(".json"))
        .ok_or_else(|| AppError::Connector(format!("unsupported Feishu path `{source_path}`")))?;
    let (file_type, token) = value
        .split_once('/')
        .ok_or_else(|| AppError::Connector(format!("unsupported Feishu path `{source_path}`")))?;

    Ok((file_type.to_owned(), token.to_owned()))
}

fn feishu_code_ok(response: &Value, context: &str) -> AppResult<()> {
    let code = response.get("code").and_then(Value::as_i64).unwrap_or(0);
    if code == 0 {
        return Ok(());
    }

    let message = response
        .get("msg")
        .and_then(Value::as_str)
        .unwrap_or("unknown error");
    Err(AppError::Connector(format!(
        "{context}: Feishu API returned code {code}: {message}"
    )))
}

fn require_non_empty(field: &str, value: &str) -> AppResult<()> {
    if value.trim().is_empty() {
        return Err(AppError::Validation(format!(
            "feishu {field} must not be empty"
        )));
    }

    Ok(())
}

fn string_field<'a>(file: &'a Value, fields: &[&str], context: &str) -> AppResult<&'a str> {
    fields
        .iter()
        .find_map(|field| file.get(*field).and_then(Value::as_str))
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::Connector(format!("{context} missing {}", fields.join("/"))))
}

fn optional_u64(file: &Value, fields: &[&str]) -> Option<u64> {
    fields.iter().find_map(|field| match file.get(*field) {
        Some(Value::Number(number)) => number.as_u64(),
        Some(Value::String(value)) => value.parse::<u64>().ok(),
        _ => None,
    })
}

fn parse_feishu_time(value: &str) -> Option<DateTime<Utc>> {
    if let Ok(seconds) = value.parse::<i64>() {
        return DateTime::from_timestamp(seconds, 0);
    }

    DateTime::parse_from_rfc3339(value)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .ok()
}

trait JsonRequestExt {
    fn send_json(self, context: &'static str) -> ConnectorFuture<'static, Value>;
}

impl JsonRequestExt for RequestBuilder {
    fn send_json(self, context: &'static str) -> ConnectorFuture<'static, Value> {
        async move {
            let response = self
                .send()
                .await
                .map_err(|error| reqwest_error(context, &error))?;
            let status = response.status();
            let body = response
                .text()
                .await
                .map_err(|error| reqwest_error(format!("read {context} response"), &error))?;
            if !status.is_success() {
                return Err(connector_error(
                    format!("{context}: Feishu API returned HTTP {status}: {body}"),
                    is_transient_http_status(status),
                ));
            }

            serde_json::from_str(&body)
                .map_err(|error| AppError::Connector(format!("decode {context} response: {error}")))
        }
        .boxed()
    }
}

fn reqwest_error(context: impl Into<String>, error: &reqwest::Error) -> AppError {
    let transient = error.is_timeout()
        || error.is_connect()
        || error.is_body()
        || error.status().is_some_and(is_transient_http_status);
    connector_error(format!("{}: {error}", context.into()), transient)
}

const fn connector_error(message: String, transient: bool) -> AppError {
    if transient {
        AppError::ConnectorTransient(message)
    } else {
        AppError::Connector(message)
    }
}

fn is_transient_http_status(status: StatusCode) -> bool {
    status == StatusCode::REQUEST_TIMEOUT
        || status == StatusCode::TOO_MANY_REQUESTS
        || status.is_server_error()
}
