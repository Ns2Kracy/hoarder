#![allow(clippy::derive_partial_eq_without_eq, clippy::future_not_send)]

use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "sync_job")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub source_id: i64,
    pub name: String,
    pub enabled: bool,
    pub schedule_kind: String,
    pub schedule_interval_seconds: Option<i64>,
    pub status: String,
    pub cursor: Option<String>,
    pub last_run_at: Option<DateTimeUtc>,
    pub last_run_status: Option<String>,
    pub last_run_id: Option<i64>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

impl ActiveModelBehavior for ActiveModel {}
