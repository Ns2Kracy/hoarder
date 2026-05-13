#![allow(clippy::derive_partial_eq_without_eq, clippy::future_not_send)]

use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "sync_job")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub source_id: Uuid,
    pub name: String,
    pub enabled: bool,
    pub status: String,
    pub cursor: Option<String>,
    pub last_run_at: Option<DateTimeUtc>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
    #[sea_orm(belongs_to, from = "source_id", to = "id")]
    pub source: HasOne<super::source::Entity>,
    #[sea_orm(has_many)]
    pub runs: HasMany<super::sync_run::Entity>,
    #[sea_orm(has_many)]
    pub errors: HasMany<super::sync_error::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
