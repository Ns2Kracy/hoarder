#![allow(clippy::derive_partial_eq_without_eq, clippy::future_not_send)]

use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "sync_run")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub job_id: i64,
    pub source_id: i64,
    pub source_name: String,
    pub job_name: String,
    pub status: String,
    pub started_at: DateTimeUtc,
    pub finished_at: Option<DateTimeUtc>,
    pub processed_count: i64,
    pub synced_count: i64,
    pub skipped_count: i64,
    pub failed_count: i64,
    pub deleted_count: i64,
    pub bytes_written: i64,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

impl ActiveModelBehavior for ActiveModel {}
