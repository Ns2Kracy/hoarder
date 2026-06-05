use std::{
    collections::{BTreeMap, VecDeque},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use bytes::Bytes;
use futures::{FutureExt, stream};
use hoarder_connectors::traits::{
    ByteStream, ConnectorConfig, ConnectorFuture, ScanOutcome, ScanStream, SourceConnector,
};
use hoarder_core::{
    AppError,
    types::{
        ConnectorCapabilities, ConnectorKind, ItemRef, ItemSnapshot, ItemType, JobId, RunId,
        SourceId, SyncStatus,
    },
};
use hoarder_sync::{
    engine::{
        ConnectorRetryPolicy, SyncEngine, SyncEngineOptions, SyncJob, SyncRunStatus, SyncRunSummary,
    },
    planner::StoredItemState,
    repository::{ItemSyncOutcome, SyncRepository},
    vault_writer::VaultWriter,
};
use tokio::time::sleep;

#[tokio::test]
async fn sync_engine_records_item_failure_and_continues_run() {
    let source_id = SourceId::new();
    let job_id = JobId::new();
    let run_id = RunId::new();
    let vault_root = temp_vault_root("failure-continues");
    let connector = Arc::new(FakeConnector::new(
        source_id,
        [
            file_snapshot(source_id, "ok.txt", 2),
            file_snapshot(source_id, "fail.txt", 4),
            file_snapshot(source_id, "later.txt", 5),
        ],
        BTreeMap::from([
            ("ok.txt".to_owned(), Ok(Bytes::from_static(b"ok"))),
            (
                "fail.txt".to_owned(),
                Err("connector read failed".to_owned()),
            ),
            ("later.txt".to_owned(), Ok(Bytes::from_static(b"later"))),
        ]),
    ));
    let repository = Arc::new(FakeRepository::new(SyncJob {
        id: job_id,
        source_id,
        source_name: "Local Docs".to_owned(),
        job_name: "Docs sync".to_owned(),
        connector_kind: ConnectorKind::OpenDal,
        connector_config: connector_config(),
        scan_cursor: None,
    }));
    repository.set_next_run_id(run_id);
    let engine = SyncEngine::new(
        repository.clone(),
        Arc::new(move |_kind| Ok(connector.clone() as Arc<dyn SourceConnector>)),
        VaultWriter::new(vault_root.clone()),
    );

    let summary = engine.run_job(job_id).await.unwrap();

    assert_eq!(
        summary,
        SyncRunSummary {
            run_id,
            processed: 3,
            synced: 2,
            skipped: 0,
            failed: 1,
            deleted: 1,
            bytes_written: 7,
        }
    );
    assert_eq!(
        tokio::fs::read(vault_root.join(source_id.to_string()).join("ok.txt"))
            .await
            .unwrap(),
        b"ok"
    );
    assert_eq!(
        tokio::fs::read(vault_root.join(source_id.to_string()).join("later.txt"))
            .await
            .unwrap(),
        b"later"
    );
    assert!(
        tokio::fs::metadata(vault_root.join(source_id.to_string()).join("fail.txt"))
            .await
            .is_err()
    );

    let events = repository.events();
    assert!(events.contains(&RepoEvent::StartRun(job_id, run_id)));
    assert!(events.contains(&RepoEvent::RecordSynced("ok.txt".to_owned())));
    assert!(events.contains(&RepoEvent::RecordFailure("fail.txt".to_owned())));
    assert!(events.contains(&RepoEvent::RecordSynced("later.txt".to_owned())));
    assert_eq!(
        events.last(),
        Some(&RepoEvent::FinishRun(
            run_id,
            SyncRunStatus::CompletedWithFailures,
            summary,
            None
        ))
    );
}

#[tokio::test]
async fn sync_engine_retries_transient_scan_errors_before_failing_run() {
    let source_id = SourceId::new();
    let job_id = JobId::new();
    let run_id = RunId::new();
    let vault_root = temp_vault_root("retry-scan");
    let connector = Arc::new(
        FakeConnector::new(
            source_id,
            [file_snapshot(source_id, "ok.txt", 2)],
            BTreeMap::from([("ok.txt".to_owned(), Ok(Bytes::from_static(b"ok")))]),
        )
        .with_transient_scan_failures(2),
    );
    let repository = Arc::new(FakeRepository::new(SyncJob {
        id: job_id,
        source_id,
        source_name: "Local Docs".to_owned(),
        job_name: "Docs sync".to_owned(),
        connector_kind: ConnectorKind::OpenDal,
        connector_config: connector_config(),
        scan_cursor: None,
    }));
    repository.set_next_run_id(run_id);
    let engine = SyncEngine::with_options(
        repository.clone(),
        Arc::new({
            let connector = connector.clone();
            move |_kind| Ok(connector.clone() as Arc<dyn SourceConnector>)
        }),
        VaultWriter::new(vault_root.clone()),
        retry_options(),
    );

    let summary = engine.run_job(job_id).await.unwrap();

    assert_eq!(connector.scan_attempts(), 3);
    assert_eq!(summary.synced, 1);
    assert_eq!(summary.failed, 0);
    assert_eq!(
        tokio::fs::read(vault_root.join(source_id.to_string()).join("ok.txt"))
            .await
            .unwrap(),
        b"ok"
    );
}

#[tokio::test]
async fn sync_engine_retries_transient_stream_read_errors_for_item() {
    let source_id = SourceId::new();
    let job_id = JobId::new();
    let run_id = RunId::new();
    let vault_root = temp_vault_root("retry-stream-read");
    let connector = Arc::new(
        FakeConnector::new(
            source_id,
            [file_snapshot(source_id, "flaky.txt", 5)],
            BTreeMap::new(),
        )
        .with_read_attempts(
            "flaky.txt",
            [
                ReadAttempt::StreamTransient("temporary stream reset".to_owned()),
                ReadAttempt::Bytes(Bytes::from_static(b"flaky")),
            ],
        ),
    );
    let repository = Arc::new(FakeRepository::new(SyncJob {
        id: job_id,
        source_id,
        source_name: "Local Docs".to_owned(),
        job_name: "Docs sync".to_owned(),
        connector_kind: ConnectorKind::OpenDal,
        connector_config: connector_config(),
        scan_cursor: None,
    }));
    repository.set_next_run_id(run_id);
    let engine = SyncEngine::with_options(
        repository.clone(),
        Arc::new({
            let connector = connector.clone();
            move |_kind| Ok(connector.clone() as Arc<dyn SourceConnector>)
        }),
        VaultWriter::new(vault_root.clone()),
        retry_options(),
    );

    let summary = engine.run_job(job_id).await.unwrap();

    assert_eq!(connector.read_attempts("flaky.txt"), 2);
    assert_eq!(summary.synced, 1);
    assert_eq!(summary.failed, 0);
    assert_eq!(summary.bytes_written, 5);
    assert_eq!(
        tokio::fs::read(vault_root.join(source_id.to_string()).join("flaky.txt"))
            .await
            .unwrap(),
        b"flaky"
    );
    assert!(
        repository
            .events()
            .contains(&RepoEvent::RecordSynced("flaky.txt".to_owned()))
    );
}

#[tokio::test]
async fn sync_engine_skips_unchanged_items_and_counts_summary() {
    let source_id = SourceId::new();
    let job_id = JobId::new();
    let run_id = RunId::new();
    let snapshot = file_snapshot(source_id, "same.txt", 4);
    let vault_root = temp_vault_root("skip-unchanged");
    let connector = Arc::new(FakeConnector::new(
        source_id,
        [snapshot.clone()],
        BTreeMap::from([("same.txt".to_owned(), Ok(Bytes::from_static(b"same")))]),
    ));
    let repository = Arc::new(FakeRepository::new(SyncJob {
        id: job_id,
        source_id,
        source_name: "Local Docs".to_owned(),
        job_name: "Docs sync".to_owned(),
        connector_kind: ConnectorKind::OpenDal,
        connector_config: connector_config(),
        scan_cursor: None,
    }));
    repository.set_next_run_id(run_id);
    repository.set_item_state(StoredItemState {
        source_path: "same.txt".to_owned(),
        item_type: ItemType::File,
        size: snapshot.size,
        etag: snapshot.etag.clone(),
        modified_at: snapshot.modified_at,
        content_hash: None,
    });
    let engine = SyncEngine::new(
        repository.clone(),
        Arc::new(move |_kind| Ok(connector.clone() as Arc<dyn SourceConnector>)),
        VaultWriter::new(vault_root.clone()),
    );

    let summary = engine.run_job(job_id).await.unwrap();

    assert_eq!(
        summary,
        SyncRunSummary {
            run_id,
            processed: 1,
            synced: 0,
            skipped: 1,
            failed: 0,
            deleted: 1,
            bytes_written: 0,
        }
    );
    assert!(
        tokio::fs::metadata(vault_root.join(source_id.to_string()).join("same.txt"))
            .await
            .is_err()
    );
    assert!(
        repository
            .events()
            .contains(&RepoEvent::RecordSkipped("same.txt".to_owned()))
    );
}

#[tokio::test]
async fn sync_engine_marks_unseen_previous_items_deleted_without_removing_local_files() {
    let source_id = SourceId::new();
    let job_id = JobId::new();
    let run_id = RunId::new();
    let vault_root = temp_vault_root("mark-deleted");
    let local_path = vault_root.join(source_id.to_string()).join("old.txt");
    tokio::fs::create_dir_all(local_path.parent().unwrap())
        .await
        .unwrap();
    tokio::fs::write(&local_path, b"keep me").await.unwrap();
    let connector = Arc::new(FakeConnector::new(
        source_id,
        [file_snapshot(source_id, "current.txt", 7)],
        BTreeMap::from([("current.txt".to_owned(), Ok(Bytes::from_static(b"current")))]),
    ));
    let repository = Arc::new(FakeRepository::new(SyncJob {
        id: job_id,
        source_id,
        source_name: "Local Docs".to_owned(),
        job_name: "Docs sync".to_owned(),
        connector_kind: ConnectorKind::OpenDal,
        connector_config: connector_config(),
        scan_cursor: None,
    }));
    repository.set_next_run_id(run_id);
    let engine = SyncEngine::new(
        repository.clone(),
        Arc::new(move |_kind| Ok(connector.clone() as Arc<dyn SourceConnector>)),
        VaultWriter::new(vault_root.clone()),
    );

    let summary = engine.run_job(job_id).await.unwrap();

    assert_eq!(summary.processed, 1);
    assert_eq!(summary.synced, 1);
    assert_eq!(summary.failed, 0);
    assert_eq!(tokio::fs::read(&local_path).await.unwrap(), b"keep me");
    assert!(
        repository
            .events()
            .contains(&RepoEvent::MarkMissingItemsDeleted(run_id, source_id))
    );
}

#[tokio::test]
async fn sync_engine_respects_file_concurrency_for_reads() {
    let source_id = SourceId::new();
    let job_id = JobId::new();
    let run_id = RunId::new();
    let vault_root = temp_vault_root("file-concurrency");
    let probe = Arc::new(ReadConcurrencyProbe::default());
    let connector = Arc::new(
        FakeConnector::new(
            source_id,
            [
                file_snapshot(source_id, "one.txt", 3),
                file_snapshot(source_id, "two.txt", 3),
                file_snapshot(source_id, "three.txt", 5),
            ],
            BTreeMap::from([
                ("one.txt".to_owned(), Ok(Bytes::from_static(b"one"))),
                ("two.txt".to_owned(), Ok(Bytes::from_static(b"two"))),
                ("three.txt".to_owned(), Ok(Bytes::from_static(b"three"))),
            ]),
        )
        .with_read_probe(Arc::clone(&probe)),
    );
    let repository = Arc::new(FakeRepository::new(SyncJob {
        id: job_id,
        source_id,
        source_name: "Local Docs".to_owned(),
        job_name: "Docs sync".to_owned(),
        connector_kind: ConnectorKind::OpenDal,
        connector_config: connector_config(),
        scan_cursor: None,
    }));
    repository.set_next_run_id(run_id);
    let engine = SyncEngine::with_options(
        repository,
        Arc::new(move |_kind| Ok(connector.clone() as Arc<dyn SourceConnector>)),
        VaultWriter::new(vault_root),
        SyncEngineOptions::new(2),
    );

    let summary = engine.run_job(job_id).await.unwrap();

    assert_eq!(summary.processed, 3);
    assert_eq!(summary.synced, 3);
    assert!(probe.max_active() >= 2);
}

#[tokio::test]
async fn sync_engine_preserves_processed_counts_when_scan_errors_after_items() {
    let source_id = SourceId::new();
    let job_id = JobId::new();
    let run_id = RunId::new();
    let vault_root = temp_vault_root("scan-error-after-items");
    let connector = Arc::new(FakeConnector::with_scan_error_after(
        source_id,
        [
            file_snapshot(source_id, "ok.txt", 2),
            file_snapshot(source_id, "fail.txt", 4),
        ],
        "scan failed after partial results",
        BTreeMap::from([
            ("ok.txt".to_owned(), Ok(Bytes::from_static(b"ok"))),
            (
                "fail.txt".to_owned(),
                Err("connector read failed".to_owned()),
            ),
        ]),
    ));
    let repository = Arc::new(FakeRepository::new(SyncJob {
        id: job_id,
        source_id,
        source_name: "Local Docs".to_owned(),
        job_name: "Docs sync".to_owned(),
        connector_kind: ConnectorKind::OpenDal,
        connector_config: connector_config(),
        scan_cursor: None,
    }));
    repository.set_next_run_id(run_id);
    let engine = SyncEngine::new(
        repository.clone(),
        Arc::new(move |_kind| Ok(connector.clone() as Arc<dyn SourceConnector>)),
        VaultWriter::new(vault_root.clone()),
    );

    let error = engine
        .run_job(job_id)
        .await
        .expect_err("scan error should fail the run");

    assert!(
        error
            .to_string()
            .contains("scan failed after partial results"),
        "{error}"
    );
    assert_eq!(
        tokio::fs::read(vault_root.join(source_id.to_string()).join("ok.txt"))
            .await
            .unwrap(),
        b"ok"
    );
    let expected_summary = SyncRunSummary {
        run_id,
        processed: 2,
        synced: 1,
        skipped: 0,
        failed: 1,
        deleted: 0,
        bytes_written: 2,
    };
    assert_eq!(
        repository.events().last(),
        Some(&RepoEvent::FinishRun(
            run_id,
            SyncRunStatus::Failed,
            expected_summary,
            None
        ))
    );
}

#[tokio::test]
async fn sync_engine_persists_connector_next_cursor_after_successful_scan() {
    let source_id = SourceId::new();
    let job_id = JobId::new();
    let run_id = RunId::new();
    let vault_root = temp_vault_root("next-cursor");
    let connector = Arc::new(
        FakeConnector::new(
            source_id,
            [file_snapshot(source_id, "cursor.txt", 6)],
            BTreeMap::from([("cursor.txt".to_owned(), Ok(Bytes::from_static(b"cursor")))]),
        )
        .with_next_cursor("cursor-2"),
    );
    let repository = Arc::new(FakeRepository::new(SyncJob {
        id: job_id,
        source_id,
        source_name: "Local Docs".to_owned(),
        job_name: "Docs sync".to_owned(),
        connector_kind: ConnectorKind::OpenDal,
        connector_config: connector_config(),
        scan_cursor: Some("cursor-1".to_owned()),
    }));
    repository.set_next_run_id(run_id);
    let engine = SyncEngine::new(
        repository.clone(),
        Arc::new(move |_kind| Ok(connector.clone() as Arc<dyn SourceConnector>)),
        VaultWriter::new(vault_root),
    );

    let summary = engine.run_job(job_id).await.unwrap();

    assert_eq!(summary.run_id, run_id);
    assert_eq!(
        repository.events().last(),
        Some(&RepoEvent::FinishRun(
            run_id,
            SyncRunStatus::Completed,
            summary,
            Some("cursor-2".to_owned())
        ))
    );
}

#[derive(Debug)]
struct FakeConnector {
    source_id: SourceId,
    snapshots: Vec<ScanEvent>,
    reads: Mutex<BTreeMap<String, VecDeque<ReadAttempt>>>,
    read_attempts: Mutex<BTreeMap<String, usize>>,
    read_probe: Option<Arc<ReadConcurrencyProbe>>,
    next_cursor: Option<String>,
    transient_scan_failures: AtomicUsize,
    scan_attempts: AtomicUsize,
}

impl FakeConnector {
    fn new<const N: usize>(
        source_id: SourceId,
        snapshots: [ItemSnapshot; N],
        reads: BTreeMap<String, Result<Bytes, String>>,
    ) -> Self {
        Self {
            source_id,
            snapshots: snapshots.into_iter().map(ScanEvent::Snapshot).collect(),
            reads: Mutex::new(read_attempts_from_legacy(reads)),
            read_attempts: Mutex::new(BTreeMap::new()),
            read_probe: None,
            next_cursor: None,
            transient_scan_failures: AtomicUsize::new(0),
            scan_attempts: AtomicUsize::new(0),
        }
    }

    fn with_read_probe(mut self, probe: Arc<ReadConcurrencyProbe>) -> Self {
        self.read_probe = Some(probe);
        self
    }

    fn with_next_cursor(mut self, next_cursor: &str) -> Self {
        self.next_cursor = Some(next_cursor.to_owned());
        self
    }

    fn with_transient_scan_failures(self, failures: usize) -> Self {
        self.transient_scan_failures
            .store(failures, Ordering::SeqCst);
        self
    }

    fn with_read_attempts<const N: usize>(
        self,
        source_path: &str,
        attempts: [ReadAttempt; N],
    ) -> Self {
        self.reads.lock().unwrap().insert(
            source_path.to_owned(),
            attempts.into_iter().collect::<VecDeque<_>>(),
        );
        self
    }

    fn scan_attempts(&self) -> usize {
        self.scan_attempts.load(Ordering::SeqCst)
    }

    fn read_attempts(&self, source_path: &str) -> usize {
        self.read_attempts
            .lock()
            .unwrap()
            .get(source_path)
            .copied()
            .unwrap_or_default()
    }

    fn with_scan_error_after<const N: usize>(
        source_id: SourceId,
        snapshots: [ItemSnapshot; N],
        error: &str,
        reads: BTreeMap<String, Result<Bytes, String>>,
    ) -> Self {
        let mut snapshots: Vec<ScanEvent> =
            snapshots.into_iter().map(ScanEvent::Snapshot).collect();
        snapshots.push(ScanEvent::Error(error.to_owned()));

        Self {
            source_id,
            snapshots,
            reads: Mutex::new(read_attempts_from_legacy(reads)),
            read_attempts: Mutex::new(BTreeMap::new()),
            read_probe: None,
            next_cursor: None,
            transient_scan_failures: AtomicUsize::new(0),
            scan_attempts: AtomicUsize::new(0),
        }
    }
}

impl SourceConnector for FakeConnector {
    fn kind(&self) -> ConnectorKind {
        ConnectorKind::OpenDal
    }

    fn validate<'a>(
        &'a self,
        _config: &'a ConnectorConfig,
    ) -> ConnectorFuture<'a, ConnectorCapabilities> {
        async move { Ok(ConnectorCapabilities::default()) }.boxed()
    }

    fn scan<'a>(
        &'a self,
        _config: &'a ConnectorConfig,
        _cursor: Option<&'a str>,
    ) -> ConnectorFuture<'a, ScanOutcome> {
        async move {
            self.scan_attempts.fetch_add(1, Ordering::SeqCst);
            if self
                .transient_scan_failures
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |remaining| {
                    remaining.checked_sub(1)
                })
                .is_ok()
            {
                return Err(AppError::ConnectorTransient(
                    "transient scan failed".to_owned(),
                ));
            }

            let snapshots = self.snapshots.clone().into_iter().map(|event| match event {
                ScanEvent::Snapshot(snapshot) => Ok(snapshot),
                ScanEvent::Error(message) => Err(AppError::Connector(message)),
            });

            let items = Box::pin(stream::iter(snapshots)) as ScanStream;
            Ok(ScanOutcome::new(items, self.next_cursor.clone()))
        }
        .boxed()
    }

    fn read<'a>(
        &'a self,
        _config: &'a ConnectorConfig,
        item_ref: &'a ItemRef,
    ) -> ConnectorFuture<'a, ByteStream> {
        async move {
            assert_eq!(item_ref.source_id, self.source_id);
            if let Some(probe) = &self.read_probe {
                probe.observe().await;
            }
            *self
                .read_attempts
                .lock()
                .unwrap()
                .entry(item_ref.source_path.clone())
                .or_default() += 1;
            let attempt = self
                .reads
                .lock()
                .unwrap()
                .get_mut(&item_ref.source_path)
                .and_then(VecDeque::pop_front)
                .expect("fake read attempt must exist");
            match attempt {
                ReadAttempt::Bytes(bytes) => Ok(Box::pin(stream::iter([Ok(bytes)])) as ByteStream),
                ReadAttempt::StreamPermanent(message) => {
                    Ok(Box::pin(stream::iter([Err(AppError::Connector(message))])) as ByteStream)
                }
                ReadAttempt::StreamTransient(message) => Ok(Box::pin(stream::iter([Err(
                    AppError::ConnectorTransient(message),
                )])) as ByteStream),
            }
        }
        .boxed()
    }
}

fn read_attempts_from_legacy(
    reads: BTreeMap<String, Result<Bytes, String>>,
) -> BTreeMap<String, VecDeque<ReadAttempt>> {
    reads
        .into_iter()
        .map(|(source_path, result)| {
            let attempt = match result {
                Ok(bytes) => ReadAttempt::Bytes(bytes),
                Err(message) => ReadAttempt::StreamPermanent(message),
            };
            (source_path, VecDeque::from([attempt]))
        })
        .collect()
}

#[derive(Clone, Debug)]
enum ReadAttempt {
    Bytes(Bytes),
    StreamPermanent(String),
    StreamTransient(String),
}

#[derive(Clone, Debug)]
enum ScanEvent {
    Snapshot(ItemSnapshot),
    Error(String),
}

#[derive(Debug, Default)]
struct ReadConcurrencyProbe {
    active: AtomicUsize,
    max_active: AtomicUsize,
}

impl ReadConcurrencyProbe {
    async fn observe(&self) {
        let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.max_active.fetch_max(active, Ordering::SeqCst);
        sleep(Duration::from_millis(25)).await;
        self.active.fetch_sub(1, Ordering::SeqCst);
    }

    fn max_active(&self) -> usize {
        self.max_active.load(Ordering::SeqCst)
    }
}

struct FakeRepository {
    job: SyncJob,
    next_run_ids: Mutex<VecDeque<RunId>>,
    item_states: Mutex<BTreeMap<String, StoredItemState>>,
    events: Mutex<Vec<RepoEvent>>,
}

impl FakeRepository {
    const fn new(job: SyncJob) -> Self {
        Self {
            job,
            next_run_ids: Mutex::new(VecDeque::new()),
            item_states: Mutex::new(BTreeMap::new()),
            events: Mutex::new(Vec::new()),
        }
    }

    fn set_next_run_id(&self, run_id: RunId) {
        self.next_run_ids.lock().unwrap().push_back(run_id);
    }

    fn set_item_state(&self, state: StoredItemState) {
        self.item_states
            .lock()
            .unwrap()
            .insert(state.source_path.clone(), state);
    }

    fn events(&self) -> Vec<RepoEvent> {
        self.events.lock().unwrap().clone()
    }
}

impl SyncRepository for FakeRepository {
    fn load_job(&self, job_id: JobId) -> ConnectorFuture<'_, SyncJob> {
        async move {
            assert_eq!(job_id, self.job.id);
            Ok(self.job.clone())
        }
        .boxed()
    }

    fn start_run<'a>(&'a self, job: &'a SyncJob) -> ConnectorFuture<'a, RunId> {
        async move {
            let run_id = self
                .next_run_ids
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_default();
            self.events
                .lock()
                .unwrap()
                .push(RepoEvent::StartRun(job.id, run_id));
            Ok(run_id)
        }
        .boxed()
    }

    fn item_state<'a>(
        &'a self,
        _source_id: SourceId,
        source_path: &'a str,
    ) -> ConnectorFuture<'a, Option<StoredItemState>> {
        async move { Ok(self.item_states.lock().unwrap().get(source_path).cloned()) }.boxed()
    }

    fn record_item_outcome(
        &self,
        _run_id: RunId,
        outcome: ItemSyncOutcome,
    ) -> ConnectorFuture<'_, ()> {
        async move {
            let event = match outcome.status {
                SyncStatus::Synced => RepoEvent::RecordSynced(outcome.source_path),
                SyncStatus::Skipped => RepoEvent::RecordSkipped(outcome.source_path),
                SyncStatus::Failed => RepoEvent::RecordFailure(outcome.source_path),
                other => panic!("unexpected item outcome status: {other:?}"),
            };
            self.events.lock().unwrap().push(event);
            Ok(())
        }
        .boxed()
    }

    fn mark_missing_items_deleted(
        &self,
        run_id: RunId,
        source_id: SourceId,
    ) -> ConnectorFuture<'_, u64> {
        async move {
            assert_eq!(source_id, self.job.source_id);
            self.events
                .lock()
                .unwrap()
                .push(RepoEvent::MarkMissingItemsDeleted(run_id, source_id));
            Ok(1)
        }
        .boxed()
    }

    fn finish_run(
        &self,
        run_id: RunId,
        status: SyncRunStatus,
        summary: SyncRunSummary,
        next_cursor: Option<String>,
    ) -> ConnectorFuture<'_, ()> {
        async move {
            self.events.lock().unwrap().push(RepoEvent::FinishRun(
                run_id,
                status,
                summary,
                next_cursor,
            ));
            Ok(())
        }
        .boxed()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum RepoEvent {
    StartRun(JobId, RunId),
    RecordSynced(String),
    RecordSkipped(String),
    RecordFailure(String),
    MarkMissingItemsDeleted(RunId, SourceId),
    FinishRun(RunId, SyncRunStatus, SyncRunSummary, Option<String>),
}

fn file_snapshot(source_id: SourceId, source_path: &str, size: u64) -> ItemSnapshot {
    ItemSnapshot {
        source_id,
        source_path: source_path.to_owned(),
        item_type: ItemType::File,
        size: Some(size),
        etag: Some(format!("{source_path}-etag")),
        modified_at: None,
        content_hash: None,
        metadata_json: None,
    }
}

fn connector_config() -> ConnectorConfig {
    ConnectorConfig::OpenDal {
        service: "fs".to_owned(),
        options: BTreeMap::new(),
    }
}

fn retry_options() -> SyncEngineOptions {
    SyncEngineOptions::new(1).with_connector_retry_policy(ConnectorRetryPolicy::new(
        3,
        Duration::ZERO,
        Duration::ZERO,
    ))
}

fn temp_vault_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("hoarder-sync-engine-{name}-{}", SourceId::new()));
    std::fs::remove_dir_all(&root).ok();
    root
}
