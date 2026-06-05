#![allow(clippy::derive_partial_eq_without_eq, clippy::future_not_send)]

use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "sync_item")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub source_id: i64,
    pub last_run_id: Option<i64>,
    pub source_path: String,
    pub item_type: String,
    pub status: String,
    pub size: Option<i64>,
    pub etag: Option<String>,
    pub modified_at: Option<DateTimeUtc>,
    pub content_hash: Option<String>,
    pub local_path: Option<String>,
    pub metadata_json: Option<Json>,
    pub last_seen_at: DateTimeUtc,
    pub synced_at: Option<DateTimeUtc>,
    pub deleted_on_source_at: Option<DateTimeUtc>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

impl ActiveModelBehavior for ActiveModel {}
