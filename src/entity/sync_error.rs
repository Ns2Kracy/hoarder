#![allow(clippy::derive_partial_eq_without_eq, clippy::future_not_send)]

use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "sync_error")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub source_id: Option<i64>,
    pub job_id: Option<i64>,
    pub run_id: Option<i64>,
    pub source_path: Option<String>,
    pub error_kind: String,
    pub message: String,
    pub created_at: DateTimeUtc,
}

impl ActiveModelBehavior for ActiveModel {}
