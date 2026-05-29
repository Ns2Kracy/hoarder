#![allow(dead_code)]

use std::{collections::BTreeMap, fs, path::PathBuf, sync::Arc};

use hoarder::{
    connectors::{
        opendal::source::OpenDalSourceConnector,
        traits::{ConnectorConfig, SourceConnector},
    },
    core::types::{ConnectorKind, JobId, SourceId},
    db::{
        connect_sqlite,
        repository::{
            NewSource, NewSyncJob, SeaOrmRepository, SourceRepository, SyncJobRepository,
        },
        schema::sync_schema,
    },
    sync::{engine::SyncEngine, vault_writer::VaultWriter},
};
use uuid::Uuid;

pub struct LocalSyncHarness {
    pub source_root: PathBuf,
    pub vault_root: PathBuf,
    pub source_id: SourceId,
    pub job_id: JobId,
    pub repository: Arc<SeaOrmRepository>,
    _temp: TempDir,
}

impl LocalSyncHarness {
    pub async fn new(name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let temp = TempDir::new(name);
        let source_root = temp.path.join("source");
        let vault_root = temp.path.join("vault");
        fs::create_dir_all(&source_root)?;

        let db_path = temp.path.join("hoarder.sqlite");
        let db = connect_sqlite(&db_path.to_string_lossy()).await?;
        sync_schema(&db).await?;

        let repository = Arc::new(SeaOrmRepository::new(db));
        let connector_config = fs_config(&source_root);
        let source = repository
            .create_source(NewSource {
                name: "local benchmark source".to_owned(),
                kind: ConnectorKind::OpenDal,
                config_json: serde_json::to_value(&connector_config)?,
                enabled: true,
            })
            .await?;
        let job = repository
            .create_job(NewSyncJob {
                source_id: source.id,
                name: "local benchmark job".to_owned(),
                enabled: true,
            })
            .await?;

        Ok(Self {
            source_root,
            vault_root,
            source_id: source.id,
            job_id: job.id,
            repository,
            _temp: temp,
        })
    }

    pub fn sync_engine(&self) -> SyncEngine<SeaOrmRepository> {
        let source_id = self.source_id;
        SyncEngine::new(
            Arc::clone(&self.repository),
            Arc::new(move |kind| {
                assert_eq!(kind, ConnectorKind::OpenDal);
                Ok(Arc::new(OpenDalSourceConnector::new(source_id)) as Arc<dyn SourceConnector>)
            }),
            VaultWriter::new(self.vault_root.clone()),
        )
    }
}

pub fn write_numbered_files(
    root: &std::path::Path,
    count: usize,
    bytes_per_file: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(root)?;
    let payload = "x".repeat(bytes_per_file);
    for index in 0..count {
        let shard = index % 16;
        let dir = root.join(format!("shard-{shard:02}"));
        fs::create_dir_all(&dir)?;
        fs::write(dir.join(format!("item-{index:05}.txt")), &payload)?;
    }
    Ok(())
}

pub fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
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
        let path = std::env::temp_dir().join(format!("hoarder-{name}-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();

        Self { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
