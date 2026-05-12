use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "sync_run")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub job_id: Uuid,
    pub source_id: Uuid,
    pub status: String,
    pub started_at: DateTimeUtc,
    pub finished_at: Option<DateTimeUtc>,
    pub processed_count: i64,
    pub synced_count: i64,
    pub skipped_count: i64,
    pub failed_count: i64,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
    #[sea_orm(belongs_to, from = "job_id", to = "id")]
    pub job: HasOne<super::sync_job::Entity>,
    #[sea_orm(belongs_to, from = "source_id", to = "id")]
    pub source: HasOne<super::source::Entity>,
    #[sea_orm(has_many)]
    pub items: HasMany<super::sync_item::Entity>,
    #[sea_orm(has_many)]
    pub errors: HasMany<super::sync_error::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
