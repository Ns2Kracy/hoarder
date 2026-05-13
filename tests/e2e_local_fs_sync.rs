use std::{collections::BTreeMap, fs, path::PathBuf, sync::Arc};

use camino::Utf8PathBuf;
use hoarder::{
    connectors::{
        opendal::source::OpenDalSourceConnector,
        traits::{ConnectorConfig, SourceConnector},
    },
    core::types::{ConnectorKind, SourceId},
    db::{
        connect_sqlite,
        repository::{
            NewSource, NewSyncJob, SeaOrmRepository, SourceRepository, SyncJobRepository,
        },
        schema::sync_schema,
    },
    entity::{sync_item, sync_run},
    sync::{engine::SyncEngine, vault_writer::VaultWriter},
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

#[tokio::test]
async fn e2e_local_fs_sync_writes_skips_and_marks_deleted() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = TempDir::new("e2e-local-fs-sync");
    let source_root = temp.path.join("source");
    let vault_root = Utf8PathBuf::from_path_buf(temp.path.join("vault")).unwrap();
    let db_path = temp.path.join("hoarder.sqlite");

    fs::create_dir_all(source_root.join("docs/nested"))?;
    fs::write(source_root.join("docs/readme.md"), "hello")?;
    fs::write(source_root.join("docs/nested/guide.txt"), "nested")?;

    let db = connect_sqlite(&db_path.to_string_lossy()).await?;
    sync_schema(&db).await?;

    let repository = Arc::new(SeaOrmRepository::new(db.clone()));
    let connector_config = fs_config(&source_root);
    let source = repository
        .create_source(NewSource {
            name: "local docs".to_owned(),
            kind: ConnectorKind::OpenDal,
            config_json: serde_json::to_value(&connector_config)?,
            enabled: true,
        })
        .await?;
    let job = repository
        .create_job(NewSyncJob {
            source_id: source.id,
            name: "default sync".to_owned(),
            enabled: true,
        })
        .await?;

    let engine = sync_engine(repository.clone(), source.id, vault_root.clone());

    let first = engine.run_job(job.id).await?;

    assert_eq!(first.failed, 0);
    assert!(first.synced >= 2);
    assert_eq!(
        tokio::fs::read(
            vault_root
                .join(source.id.to_string())
                .join("docs/readme.md")
        )
        .await?,
        b"hello"
    );
    assert_eq!(
        tokio::fs::read(
            vault_root
                .join(source.id.to_string())
                .join("docs/nested/guide.txt")
        )
        .await?,
        b"nested"
    );

    let first_run = sync_run::Entity::find_by_id(first.run_id.as_uuid())
        .one(&db)
        .await?
        .expect("first sync run row exists");
    assert_eq!(first_run.status, "completed");
    assert_eq!(first_run.processed_count, first.processed.cast_signed());
    assert_eq!(first_run.synced_count, first.synced.cast_signed());
    assert_eq!(first_run.skipped_count, first.skipped.cast_signed());
    assert_eq!(first_run.failed_count, 0);
    assert!(first_run.finished_at.is_some());

    let readme = sync_item(source.id, "docs/readme.md", &db).await?;
    assert_eq!(readme.status, "synced");
    let readme_vault_path = vault_root
        .join(source.id.to_string())
        .join("docs/readme.md");
    assert_eq!(
        readme.local_path.as_deref(),
        Some(readme_vault_path.as_str())
    );

    let second = engine.run_job(job.id).await?;

    assert_eq!(second.failed, 0);
    assert!(second.skipped >= 2);
    assert_eq!(
        sync_item(source.id, "docs/readme.md", &db).await?.status,
        "skipped"
    );
    assert_eq!(
        sync_item(source.id, "docs/nested/guide.txt", &db)
            .await?
            .status,
        "skipped"
    );

    tokio::fs::remove_file(source_root.join("docs/nested/guide.txt")).await?;
    let third = engine.run_job(job.id).await?;

    assert_eq!(third.failed, 0);
    assert_eq!(
        tokio::fs::read(
            vault_root
                .join(source.id.to_string())
                .join("docs/nested/guide.txt")
        )
        .await?,
        b"nested"
    );
    let deleted = sync_item(source.id, "docs/nested/guide.txt", &db).await?;
    assert_eq!(deleted.status, "deleted_on_source");
    assert!(deleted.deleted_on_source_at.is_some());

    Ok(())
}

fn sync_engine(
    repository: Arc<SeaOrmRepository>,
    source_id: SourceId,
    vault_root: Utf8PathBuf,
) -> SyncEngine<SeaOrmRepository> {
    SyncEngine::new(
        repository,
        Arc::new(move |kind| {
            assert_eq!(kind, ConnectorKind::OpenDal);
            Ok(Arc::new(OpenDalSourceConnector::new(source_id)) as Arc<dyn SourceConnector>)
        }),
        VaultWriter::new(vault_root),
    )
}

async fn sync_item(
    source_id: SourceId,
    source_path: &str,
    db: &sea_orm::DatabaseConnection,
) -> Result<sync_item::Model, Box<dyn std::error::Error>> {
    Ok(sync_item::Entity::find()
        .filter(sync_item::Column::SourceId.eq(source_id.as_uuid()))
        .filter(sync_item::Column::SourcePath.eq(source_path))
        .one(db)
        .await?
        .unwrap_or_else(|| panic!("sync item row exists for {source_path}")))
}

fn fs_config(root: &std::path::Path) -> ConnectorConfig {
    ConnectorConfig::OpenDal {
        service: "fs".to_owned(),
        options: BTreeMap::from([("root".to_owned(), root.to_string_lossy().into_owned())]),
    }
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("hoarder-{name}-{}", Uuid::now_v7()));
        fs::create_dir_all(&path).unwrap();

        Self { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
