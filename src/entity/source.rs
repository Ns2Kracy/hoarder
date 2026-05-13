#![allow(clippy::derive_partial_eq_without_eq, clippy::future_not_send)]

use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "source")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub name: String,
    pub kind: String,
    pub config_json: Json,
    pub enabled: bool,
    pub last_check_status: Option<String>,
    pub last_checked_at: Option<DateTimeUtc>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
    #[sea_orm(has_many)]
    pub jobs: HasMany<super::sync_job::Entity>,
    #[sea_orm(has_many)]
    pub runs: HasMany<super::sync_run::Entity>,
    #[sea_orm(has_many)]
    pub items: HasMany<super::sync_item::Entity>,
    #[sea_orm(has_many)]
    pub errors: HasMany<super::sync_error::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
