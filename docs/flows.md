# Hoarder Flows

日期：2026-05-29
状态：当前产品与技术流程说明
读者：产品负责人、工程师、测试负责人、后续维护 agent

## 1. 产品主流程

```mermaid
flowchart LR
    Install[安装或构建 Hoarder] --> Serve[启动 hoarder serve]
    Serve --> Open[打开 Web 控制台或使用 CLI]
    Open --> CreateSource[创建 source]
    CreateSource --> TestSource[测试 source]
    TestSource --> CreateJob[创建 sync job]
    CreateJob --> RunNow[手动运行 job]
    RunNow --> Inspect[查看 run、items、errors]
    Inspect --> Schedule{interval job enabled?}
    Schedule -- yes --> AutoRun[serve 模式自动调度]
    Schedule -- no --> ManualOnly[按需手动运行]
    AutoRun --> Vault[从本地 vault 读取同步文件]
    ManualOnly --> Vault
```

产品判断点：

- Source 未测试也可以保存，但 UI 应提示健康状态为 `untested`。
- Job 被禁用时不允许运行，展示为 paused/disabled 语义。
- Run 失败后用户应能从 run detail 进入 error 和 item 视图定位问题。

## 2. Source 创建与测试流程

```mermaid
sequenceDiagram
    actor User
    participant UI as Web/CLI
    participant API as Axum routes
    participant SourceService as source_service
    participant Repo as SeaOrmRepository
    participant Connector as OpenDalSourceConnector
    participant DB as SQLite

    User->>UI: 输入 source 名称、service、options
    UI->>API: POST /api/sources 或 CLI source add
    API->>SourceService: create_source(request)
    SourceService->>Repo: create_source(NewSource)
    Repo->>DB: insert source
    DB-->>Repo: source row
    Repo-->>SourceService: SourceRecord
    SourceService-->>API: SourceDto(secret redacted)
    API-->>UI: created source

    User->>UI: Test source
    UI->>API: POST /api/sources/{id}/test
    API->>SourceService: test_source(id)
    SourceService->>Repo: load_source(id)
    Repo->>DB: select source
    DB-->>Repo: source row
    SourceService->>Connector: validate(config)
    Connector-->>SourceService: capabilities or error
    SourceService->>Repo: update last_check_status
    Repo->>DB: update source health
    SourceService-->>API: SourceTestResponse or error
    API-->>UI: health result
```

技术要点：

- API 返回的 connector config 必须脱敏。
- 更新 source 时，如果用户提交的是 redacted secret，占位值会合并回已有 secret。
- `validate` 当前返回 capability，不写入 vault，也不启动 sync。

## 3. Job 创建与手动运行流程

```mermaid
sequenceDiagram
    actor User
    participant UI as Web/CLI
    participant JobService as job_service
    participant Repo as SeaOrmRepository
    participant Engine as SyncEngine
    participant Connector as SourceConnector
    participant Vault as VaultWriter
    participant DB as SQLite

    User->>UI: 创建 job
    UI->>JobService: create_job(source_id, schedule, enabled)
    JobService->>Repo: ensure source exists + insert sync_job
    Repo->>DB: insert sync_job
    DB-->>Repo: sync_job row
    Repo-->>JobService: JobDto
    JobService-->>UI: job created

    User->>UI: Run now
    UI->>JobService: run_job(job_id)
    JobService->>DB: atomic mark job running
    alt disabled or already running
        JobService-->>UI: 422 or 409
    else accepted
        JobService->>Engine: run_job(job_id)
        Engine->>Repo: load_job + start_run
        Repo->>DB: select job/source + insert sync_run
        Engine->>Connector: scan(config, cursor)
        loop for each ItemSnapshot
            Engine->>Repo: item_state(source_id, source_path)
            Engine->>Connector: read(item_ref) when sync needed
            Engine->>Vault: write byte stream to vault
            Engine->>Repo: record_item_outcome
        end
        Engine->>Repo: mark_missing_items_deleted
        Engine->>Repo: finish_run(summary)
        Repo->>DB: update run + job last_run fields
        Engine-->>JobService: summary
        JobService-->>UI: run_id + status
    end
```

产品要点：

- 手动 run 与 scheduler run 使用同一条业务路径。
- Run 成功但有 item 失败时，run 状态是 `completed_with_failures`，用户仍可审计已成功同步的 item。
- Connector 级失败会使整个 run 失败。

## 4. 同步引擎 item 决策流程

```mermaid
flowchart TD
    Snapshot[Receive ItemSnapshot] --> LoadState[Load stored sync_item state]
    LoadState --> Exists{stored item exists?}
    Exists -- no --> Sync[PlanDecision::Sync]
    Exists -- yes --> TypeChanged{item_type changed?}
    TypeChanged -- yes --> Sync
    TypeChanged -- no --> HasEtag{both have etag?}
    HasEtag -- yes --> EtagChanged{etag changed?}
    EtagChanged -- yes --> Sync
    EtagChanged -- no --> MetaChanged{size or modified_at changed?}
    MetaChanged -- yes --> Sync
    MetaChanged -- no --> Skip[PlanDecision::Skip]
    HasEtag -- no --> SizeTimeChanged{size or modified_at changed?}
    SizeTimeChanged -- yes --> Sync
    SizeTimeChanged -- no --> BothMatched{size and modified_at matched?}
    BothMatched -- yes --> Skip
    BothMatched -- no --> HashKnown{both content_hash present?}
    HashKnown -- yes --> HashEqual{hash equal?}
    HashEqual -- yes --> Skip
    HashEqual -- no --> Sync
    HashKnown -- no --> Sync

    Sync --> IsDirectory{directory?}
    IsDirectory -- yes --> RecordDir[Record synced directory outcome]
    IsDirectory -- no --> Read[Read source byte stream]
    Read --> Write[VaultWriter writes file]
    Write --> RecordFile[Record synced file outcome]
    Skip --> RecordSkip[Record skipped item outcome]
```

源端删除不在单个 snapshot 决策中完成，而是在 scan 结束后统一执行：当前 run 没有看到、且此前未标记删除的同 source item，会被标记为 `deleted_on_source`。

## 5. Vault 写入流程

```mermaid
flowchart TD
    ItemRef[ItemRef source_id + source_path] --> Normalize[normalize_source_path]
    Normalize --> PathValid{path safe?}
    PathValid -- no --> PathError[return AppError::Path]
    PathValid -- yes --> Target[Build vault/{source_id}/normalized/path]
    Target --> Temp[Build vault/.hoarder/tmp/{uuid}-{leaf}.tmp]
    Temp --> Mkdir[Create parent directories]
    Mkdir --> Stream[Stream chunks from connector]
    Stream --> Hash[Update SHA-256 and byte count]
    Hash --> Flush[Flush temp file]
    Flush --> Rename[Atomic rename temp -> target]
    Rename --> Outcome[VaultWrite target_path + hash + bytes]
    Stream -. any error .-> Cleanup[Remove temp file best-effort]
    Cleanup --> Error[Return error]
```

安全规则：

- 空路径、绝对路径、路径穿越、NUL、Windows drive prefix 均拒绝。
- source path 第一段为 `.hoarder` 时拒绝，避免覆盖内部临时目录。
- 源端消失的文件不会触发本地文件删除。

## 6. Scheduler 流程

```mermaid
flowchart TD
    Start[hoarder serve starts] --> Spawn[spawn_interval_scheduler]
    Spawn --> Tick[Every 30 seconds]
    Tick --> LoadSettings[Load runtime settings]
    LoadSettings --> JobConcurrency{job_concurrency > 0?}
    JobConcurrency -- no --> Sleep[Wait next tick]
    JobConcurrency -- yes --> ListJobs[List jobs]
    ListJobs --> Filter[enabled + interval + not running]
    Filter --> Due{next_run_at is due?}
    Due -- no --> Sleep
    Due -- yes --> Take[Take up to job_concurrency]
    Take --> RunJob[job_service::run_job]
    RunJob --> Log{run ok?}
    Log -- yes --> Count[Increment started count]
    Log -- no --> Warn[tracing warn]
    Count --> Sleep
    Warn --> Sleep
    Sleep --> Tick
```

当前 scheduler 是单进程设计。它适合本地 `serve` 模式，不提供跨进程 advisory lock 或分布式调度语义。

## 7. Run 状态与错误流程

```mermaid
stateDiagram-v2
    [*] --> Running: start_run
    Running --> Completed: no item failures and no connector failure
    Running --> CompletedWithFailures: one or more item failures
    Running --> Failed: connector-level failure or unrecoverable run error
    Running --> Cancelled: reserved domain state
    Completed --> [*]
    CompletedWithFailures --> [*]
    Failed --> [*]
    Cancelled --> [*]
```

错误映射：

| 错误类型 | Run 影响 | API 映射 | 持久化 |
| --- | --- | --- | --- |
| JSON/path 提取错误 | 不启动 run | `400 VALIDATION_ERROR` | 无 |
| Source/job/run 不存在 | 不启动 run | `404 NOT_FOUND` | 无 |
| Job 已在运行 | 不启动 run | `409 CONFLICT` | 无 |
| Disabled job | 不启动 run | `422 UNPROCESSABLE_ENTITY` | 无 |
| Item read/write/path 失败 | run 可继续 | run detail 中展示 | `sync_item` + `sync_error` |
| Connector scan/build 失败 | run 失败 | `502 CONNECTOR_ERROR` | `sync_run` 失败摘要 |
| Database/IO 内部错误 | run 或 API 失败 | `500 INTERNAL_ERROR` | 视失败阶段而定 |

## 8. Settings 更新流程

```mermaid
sequenceDiagram
    actor User
    participant UI as Web Console
    participant API as PATCH /api/settings
    participant Settings as settings_service
    participant Repo as RuntimeSettingsRepository
    participant Logging as logging
    participant DB as app_setting

    User->>UI: 修改 job/file concurrency 或 log level
    UI->>API: PATCH /api/settings
    API->>Settings: update_settings(request)
    Settings->>Repo: patch_runtime_settings(config, patch)
    Repo->>Repo: validate concurrency > 0
    Repo->>DB: upsert app_setting rows
    DB-->>Repo: updated settings
    Repo-->>Settings: RuntimeSettings
    Settings->>Logging: set_level(log_level)
    Settings-->>API: SettingsDto
    API-->>UI: saved settings
```

只读启动配置：`database_path`、`vault_path`、`listen_addr` 来自启动配置或 CLI flag，API 会返回但不会通过 runtime settings 修改。

## 9. Web 控制台数据刷新流程

```mermaid
flowchart LR
    AppMount[App mount or refresh click] --> Load[loadConsoleData]
    Load --> Sources[GET /api/sources]
    Load --> Jobs[GET /api/jobs]
    Load --> Runs[GET /api/runs]
    Sources --> Summary[derive overview summary]
    Jobs --> Summary
    Runs --> Summary
    Summary --> Pages[Overview/Sources/Jobs/Runs/Settings]
    Sources -. API unavailable .-> Mock[Mock fallback]
    Jobs -. API unavailable .-> Mock
    Runs -. API unavailable .-> Mock
    Mock --> Pages
```

当前前端保留 mock fallback，便于本地 API 不可用时预览控制台。但产品真实状态应以 live API 为准。

## 10. AI/RAG 知识库聚合流程

```mermaid
flowchart TD
    SyncDone[Sync run finished] --> Items[Updated sync_item rows]
    Items --> Changed{new or content_hash changed?}
    Changed -- no --> NoIndex[Skip indexing]
    Changed -- yes --> ReadVault[Read local_path from vault]
    ReadVault --> Extract[Extract text and structured metadata]
    Extract --> Chunk[Split into chunks]
    Chunk --> Attach[Attach provenance]
    Attach --> StoreChunks[Store chunk metadata]
    StoreChunks --> FullText[Update full-text index]
    StoreChunks --> Embed{embedding configured?}
    Embed -- no --> SearchOnly[Text/metadata retrieval only]
    Embed -- yes --> Vectorize[Generate embeddings]
    Vectorize --> VectorIndex[Update vector index]
    FullText --> Retrieval[Hybrid retrieval API]
    VectorIndex --> Retrieval
    Retrieval --> RAG[Local RAG app or agent]

    Attach -. includes .-> Provenance[source_id, source_path, local_path, run_id, hash]
```

这个流程是规划中的 RAG 扩展路径。当前 Hoarder 已经提供前半段：source 聚合、vault 写入、metadata、hash、run 和 error 审计。后半段应作为独立 index pipeline 引入，避免同步成功与索引成功互相耦合。

RAG 场景的关键约束：

- Retrieval 返回的每段上下文必须带引用信息，可追溯到 source 和本地文件。
- Index pipeline 应按 hash 或 updated_at 增量处理，避免每次全量重建。
- Embedding provider、vector store 和 LLM gateway 应显式配置，默认保持本地优先和不外发数据。

## 11. 测试流程建议

| 流程 | 推荐验证 |
| --- | --- |
| Source 创建/测试 | API route tests、connector contract tests、source service tests |
| Job 创建/运行 | app service tests、CLI command tests、API route tests |
| Sync engine | `tests/sync_engine.rs`、`tests/sync_planner.rs`、`tests/vault_writer.rs` |
| 本地端到端 | `tests/e2e_local_fs_sync.rs` |
| Schema | `tests/db_schema.rs`、空数据库启动 smoke test |
| Web 控制台 | `cd web && bun run verify`，后续补浏览器截图回归 |
| 打包 | `cd web && bun run build && cargo build --release` |
| RAG 索引扩展 | parser/chunker 单元测试、provenance 保留测试、hash 增量索引测试、外部 embedding 禁用默认值测试 |
