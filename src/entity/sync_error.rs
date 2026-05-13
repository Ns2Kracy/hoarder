#![allow(clippy::derive_partial_eq_without_eq, clippy::future_not_send)]

use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "sync_error")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub source_id: Uuid,
    pub job_id: Option<Uuid>,
    pub run_id: Option<Uuid>,
    pub item_id: Option<Uuid>,
    pub source_path: Option<String>,
    pub error_kind: String,
    pub message: String,
    pub created_at: DateTimeUtc,
    #[sea_orm(belongs_to, from = "source_id", to = "id")]
    pub source: HasOne<super::source::Entity>,
    #[sea_orm(belongs_to, from = "job_id", to = "id")]
    pub job: HasOne<super::sync_job::Entity>,
    #[sea_orm(belongs_to, from = "run_id", to = "id")]
    pub run: HasOne<super::sync_run::Entity>,
    #[sea_orm(belongs_to, from = "item_id", to = "id")]
    pub item: HasOne<super::sync_item::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
