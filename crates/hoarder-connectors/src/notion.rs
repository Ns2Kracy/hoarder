use bytes::Bytes;
use chrono::{DateTime, Utc};
use futures::{FutureExt, stream};
use reqwest::{Client, Method, RequestBuilder, StatusCode};
use serde_json::{Value, json};

use crate::traits::{
    ByteStream, ConnectorConfig, ConnectorFuture, ScanOutcome, ScanStream, SourceConnector,
};
use hoarder_core::{
    AppError, AppResult,
    types::{ConnectorCapabilities, ConnectorKind, ItemRef, ItemSnapshot, ItemType, SourceId},
};

const DEFAULT_BASE_URL: &str = "https://api.notion.com";
const DEFAULT_NOTION_VERSION: &str = "2026-03-11";
const PAGE_SIZE: u16 = 100;

#[derive(Clone, Debug)]
pub struct NotionSourceConnector {
    source_id: SourceId,
    client: Client,
}

impl NotionSourceConnector {
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

impl SourceConnector for NotionSourceConnector {
    fn kind(&self) -> ConnectorKind {
        ConnectorKind::Notion
    }

    fn validate<'a>(
        &'a self,
        config: &'a ConnectorConfig,
    ) -> ConnectorFuture<'a, ConnectorCapabilities> {
        async move {
            let config = notion_config(config)?;
            self.request(&config, Method::GET, "/v1/users/me")
                .send_json("validate Notion connector")
                .await?;

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
            let config = notion_config(config)?;
            let response = self.scan_response(&config, cursor).await?;
            let next_cursor = next_cursor(&response)?;
            let snapshots = notion_snapshots(self.source_id, &response)?;
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
            if item_ref.item_type != ItemType::VirtualDocument {
                return Err(AppError::Connector(format!(
                    "cannot read non-document Notion item `{}`",
                    item_ref.source_path
                )));
            }

            let config = notion_config(config)?;
            let document = self.read_document(&config, item_ref).await?;
            let bytes = serde_json::to_vec_pretty(&document).map_err(|error| {
                AppError::Connector(format!("serialize Notion virtual document: {error}"))
            })?;
            let bytes = Bytes::from(bytes);

            Ok(Box::pin(stream::once(async move { Ok(bytes) })) as ByteStream)
        }
        .boxed()
    }
}

impl NotionSourceConnector {
    async fn scan_response(
        &self,
        config: &ResolvedNotionConfig<'_>,
        cursor: Option<&str>,
    ) -> AppResult<Value> {
        if let Some(data_source_id) = config.data_source_id {
            return self.query_data_source(config, data_source_id, cursor).await;
        }
        let page_id = config.page_id.ok_or_else(|| {
            AppError::Validation("notion connector requires dataSourceId or pageId".to_owned())
        })?;

        self.block_children(config, page_id, cursor).await
    }

    async fn query_data_source(
        &self,
        config: &ResolvedNotionConfig<'_>,
        data_source_id: &str,
        cursor: Option<&str>,
    ) -> AppResult<Value> {
        let mut body = json!({"page_size": PAGE_SIZE});
        if let Some(cursor) = cursor {
            body["start_cursor"] = json!(cursor);
        }

        let path = format!("/v1/data_sources/{data_source_id}/query");
        self.request(config, Method::POST, &path)
            .json(&body)
            .send_json("query Notion data source")
            .await
    }

    async fn block_children(
        &self,
        config: &ResolvedNotionConfig<'_>,
        block_id: &str,
        cursor: Option<&str>,
    ) -> AppResult<Value> {
        let path = format!("/v1/blocks/{block_id}/children");
        let mut request = self
            .request(config, Method::GET, &path)
            .query(&[("page_size", PAGE_SIZE.to_string())]);
        if let Some(cursor) = cursor {
            request = request.query(&[("start_cursor", cursor)]);
        }

        request.send_json("list Notion block children").await
    }

    async fn read_document(
        &self,
        config: &ResolvedNotionConfig<'_>,
        item_ref: &ItemRef,
    ) -> AppResult<Value> {
        if let Some(page_id) = notion_item_id(&item_ref.source_path, "notion/page/") {
            let page = self
                .get_json(config, &format!("/v1/pages/{page_id}"))
                .await?;
            let children = self.block_children(config, &page_id, None).await?;
            return Ok(json!({
                "provider": "notion",
                "kind": "page",
                "id": page_id,
                "sourcePath": item_ref.source_path,
                "page": page,
                "children": children_results(&children)?,
                "childrenHasMore": children_has_more(&children)?
            }));
        }
        if let Some(block_id) = notion_item_id(&item_ref.source_path, "notion/block/") {
            let block = self
                .get_json(config, &format!("/v1/blocks/{block_id}"))
                .await?;
            let children = self.block_children(config, &block_id, None).await?;
            return Ok(json!({
                "provider": "notion",
                "kind": "block",
                "id": block_id,
                "sourcePath": item_ref.source_path,
                "block": block,
                "children": children_results(&children)?,
                "childrenHasMore": children_has_more(&children)?
            }));
        }

        Err(AppError::Connector(format!(
            "unsupported Notion source path `{}`",
            item_ref.source_path
        )))
    }

    async fn get_json(&self, config: &ResolvedNotionConfig<'_>, path: &str) -> AppResult<Value> {
        self.request(config, Method::GET, path)
            .send_json("get Notion object")
            .await
    }

    fn request(
        &self,
        config: &ResolvedNotionConfig<'_>,
        method: Method,
        path: &str,
    ) -> RequestBuilder {
        self.client
            .request(method, format!("{}{}", config.base_url, path))
            .bearer_auth(config.token)
            .header("Notion-Version", config.version)
            .header("Accept", "application/json")
    }
}

#[derive(Clone, Debug)]
struct ResolvedNotionConfig<'a> {
    token: &'a str,
    data_source_id: Option<&'a str>,
    page_id: Option<&'a str>,
    version: &'a str,
    base_url: String,
}

fn notion_config(config: &ConnectorConfig) -> AppResult<ResolvedNotionConfig<'_>> {
    let ConnectorConfig::Notion {
        token,
        data_source_id,
        page_id,
        version,
        base_url,
    } = config
    else {
        return Err(AppError::Validation(
            "expected notion connector config".to_owned(),
        ));
    };
    if token.trim().is_empty() {
        return Err(AppError::Validation(
            "notion token must not be empty".to_owned(),
        ));
    }
    if data_source_id.as_deref().is_none_or(str::is_empty)
        && page_id.as_deref().is_none_or(str::is_empty)
    {
        return Err(AppError::Validation(
            "notion connector requires dataSourceId or pageId".to_owned(),
        ));
    }

    Ok(ResolvedNotionConfig {
        token,
        data_source_id: data_source_id.as_deref().filter(|value| !value.is_empty()),
        page_id: page_id.as_deref().filter(|value| !value.is_empty()),
        version: version.as_deref().unwrap_or(DEFAULT_NOTION_VERSION),
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
        supports_directories: false,
        supports_virtual_documents: true,
        supports_incremental_scan: true,
    }
}

fn notion_snapshots(source_id: SourceId, response: &Value) -> AppResult<Vec<ItemSnapshot>> {
    let results = response
        .get("results")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::Connector("Notion response missing results array".to_owned()))?;

    results
        .iter()
        .map(|item| notion_snapshot(source_id, item))
        .collect()
}

fn notion_snapshot(source_id: SourceId, item: &Value) -> AppResult<ItemSnapshot> {
    let id = string_field(item, "id", "Notion item")?;
    let object = item
        .get("object")
        .and_then(Value::as_str)
        .unwrap_or("block");
    let source_path = match object {
        "page" => format!("notion/page/{id}.json"),
        _ => format!("notion/block/{id}.json"),
    };
    let last_edited_time = item.get("last_edited_time").and_then(Value::as_str);

    Ok(ItemSnapshot {
        source_id,
        source_path,
        item_type: ItemType::VirtualDocument,
        size: None,
        etag: last_edited_time.map_or_else(|| Some(id.to_owned()), |value| Some(value.to_owned())),
        modified_at: last_edited_time.and_then(parse_rfc3339),
        content_hash: None,
        metadata_json: Some(json!({
            "provider": "notion",
            "object": object,
            "id": id,
            "raw": item
        })),
    })
}

fn next_cursor(response: &Value) -> AppResult<Option<String>> {
    let has_more = response
        .get("has_more")
        .and_then(Value::as_bool)
        .ok_or_else(|| AppError::Connector("Notion response missing has_more".to_owned()))?;
    if !has_more {
        return Ok(None);
    }

    response
        .get("next_cursor")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| AppError::Connector("Notion response missing next_cursor".to_owned()))
        .map(Some)
}

fn notion_item_id(source_path: &str, prefix: &str) -> Option<String> {
    source_path
        .strip_prefix(prefix)
        .and_then(|value| value.strip_suffix(".json"))
        .map(ToOwned::to_owned)
}

fn children_results(children: &Value) -> AppResult<Value> {
    let results = children
        .get("results")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            AppError::Connector("Notion children response missing results".to_owned())
        })?;

    Ok(Value::Array(results.clone()))
}

fn children_has_more(children: &Value) -> AppResult<bool> {
    children
        .get("has_more")
        .and_then(Value::as_bool)
        .ok_or_else(|| AppError::Connector("Notion children response missing has_more".to_owned()))
}

fn string_field<'a>(item: &'a Value, field: &str, context: &str) -> AppResult<&'a str> {
    item.get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::Connector(format!("{context} missing {field}")))
}

fn parse_rfc3339(value: &str) -> Option<DateTime<Utc>> {
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
                    format!("{context}: Notion API returned {status}: {body}"),
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
