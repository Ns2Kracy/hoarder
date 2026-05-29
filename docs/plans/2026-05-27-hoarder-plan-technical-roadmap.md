# Hoarder 计划书、技术方案与 Roadmap

日期：2026-05-27
状态：草案
适用范围：Hoarder 本地优先数据聚合与单向同步平台
读者：项目维护者、后端工程师、前端工程师、测试与发布负责人

## 1. 执行摘要

Hoarder 是一个本地优先的数据聚合和单向同步平台。它连接外部数据源，把内容同步到用户可直接阅读的本地 vault，并把 source、job、run、item、error 等元数据记录到本地 SQLite，方便通过 CLI、HTTP API 和 Web 控制台进行配置、运行、排障和审计。

当前项目已经完成本地 MVP 主路径：Rust 2024 后端、Axum API、SeaORM 2.0 entity-first、SQLite、OpenDAL filesystem connector、WebDAV/SFTP/S3 operator wiring、单向同步引擎、定时 job、运行时设置、Svelte 5 Web 控制台、OpenAPI、单二进制嵌入式前端和基础测试门禁。下一阶段的重点不是重写核心，而是在现有边界上补齐工程化和产品化能力：CI、发布产物、connector-specific typed config、远端 connector integration tests、可访问性和浏览器回归、增量 cursor、重试策略、搜索、标签和长期运行稳定性。

推荐路线是分三步推进：

1. **稳定化**：补齐 CI、发布流水线、测试和可访问性，确保现有 MVP 可重复构建、可安装、可回归。
2. **连接器扩展**：优先补齐 WebDAV、SFTP、S3 的 typed config、integration tests 和 NAS 预设，让 Hoarder 覆盖真实本地与私有云存储场景。
3. **知识库能力**：在可靠同步基础上加入全文搜索、标签、集合、去重和通知，把平台从“同步工具”推进为“本地数据汇聚与检索工作台”。

## 2. 项目目标

### 2.1 产品目标

- 为个人和小团队提供一个可本地运行、可审计、可恢复的数据汇聚工具。
- 将不同 source 的内容以单向方式同步到统一本地 vault，connector 只读取 source，不修改 source。
- 保持 vault 文件结构可读，让用户即使离开 Hoarder 也能直接访问同步结果。
- 通过 Web 控制台降低配置和排障成本，通过 CLI 保持自动化和脚本友好。
- 先打透本地、单用户、单进程场景，再谨慎扩展远程多用户模式。

### 2.2 工程目标

- 保持核心边界清晰：interface 层不承载业务逻辑，sync engine 不知道 HTTP、CLI 或具体 OpenDAL 类型。
- 保持严格质量门禁：Rust warnings 和 Clippy deny，前端使用 Bun verify，发布前跑完整构建。
- 保持默认安全：监听 `127.0.0.1`，敏感配置脱敏，不自动删除本地文件，路径写入受 vault 约束。
- 保持 release 简单：前端构建后嵌入 Rust 单二进制，优先支持本地离线使用。

### 2.3 成功指标

| 维度 | 指标 |
| --- | --- |
| 可用性 | 新用户能在 10 分钟内完成 filesystem source 创建、job 创建、手动同步和 run 查看 |
| 可靠性 | `cargo test`、`cargo clippy --all-targets --all-features`、`bun run verify` 和 release build 在 CI 中稳定通过 |
| 数据安全 | 路径穿越、绝对路径、`.hoarder` 保留路径写入均被拒绝；source 消失不触发本地文件删除 |
| 连接器覆盖 | filesystem、WebDAV、SFTP、S3 均具备创建、测试、同步、错误记录和 UI 配置路径 |
| 可观测性 | 每次 run 有状态、计数、错误、耗时和相关 source/job 展示字段 |
| 发布能力 | macOS、Linux、Windows 至少提供可下载 release artifact |

## 3. 当前状态

### 3.1 已完成能力

- Rust 2024 + Tokio + Axum 本地 API。
- SQLite + SeaORM 2.0 entity-first 元数据存储。
- `source`、`sync_job`、`sync_run`、`sync_item`、`sync_error`、`app_setting` 等核心实体。
- OpenDAL connector kind 和 filesystem connector。
- OpenDAL `fs`、`webdav`、`sftp`、`s3` 配置模型、校验与 operator wiring。
- source 测试、job 创建、手动 run、固定间隔调度。
- 单向同步：source 到 local vault。
- 安全 vault writer：规范化路径、拒绝路径穿越、临时文件写入、原子替换。
- 运行记录、item 状态、错误记录、计数和 SHA-256 hash。
- CLI、Axum API、Svelte Web 控制台三种操作入口。
- OpenAPI specification。
- 前端资源嵌入 Rust release binary。
- CI workflow、跨平台 release artifacts、installer 脚本、基础性能 benchmark 和本地 filesystem soak test。
- 现有测试覆盖 sync engine、API routes、CLI workflows、app services、connector contract 和 vault safety。

### 3.2 主要缺口

- WebDAV、SFTP、S3 仍缺少更贴近真实服务的 integration tests。
- 缺少可访问性检查和浏览器截图回归。
- 缺少 connector cursor、重试策略和数据库保留策略。
- 搜索、标签、集合、跨 source 去重和通知仍在产品 Roadmap 中。
- 认证、授权、多用户远程部署和自动本地删除仍应保持后置。

## 4. 范围定义

### 4.1 下一阶段范围

下一阶段聚焦“可发布、可扩展、可长期运行”的本地产品形态：

- 前端可访问性和浏览器回归验证。
- connector transient error 重试策略。
- connector cursor 和增量扫描接口。
- 数据库清理和保留策略。
- 全文搜索的技术验证和首个可用版本。

### 4.2 暂不纳入范围

- 远程多用户部署模式。
- 登录、认证、授权和租户隔离。
- 默认自动删除本地文件。
- 第三方编译插件 ABI。
- Notion 和飞书完整 connector。
- 复杂工作流自动化或规则引擎。

这些能力不是不做，而是需要建立在稳定的本地单用户同步模型、连接器模型和发布链路之后。

## 5. 产品方案

### 5.1 用户画像

1. **个人数据归档用户**
   需要把 NAS、WebDAV、S3 bucket 或本地目录中的文件定期汇总到一个本地 vault，保留可读目录结构，并能审计同步结果。

2. **开发者和技术用户**
   希望通过 CLI 和 JSON 配置自动化 source/job 管理，用脚本触发 sync run，并把 Hoarder 嵌入自己的本地工作流。

3. **小团队工具维护者**
   希望在一台本地机器或内网机器上运行同步服务，通过 Web 控制台查看 source 健康、job 状态、同步错误和运行历史。

### 5.2 核心用户旅程

```text
安装 Hoarder
  -> 启动本地服务
  -> 创建 source
  -> 测试 source
  -> 创建 manual 或 interval job
  -> 手动触发第一次 run
  -> 查看 run detail、items 和 errors
  -> 保持 serve 模式定期同步
  -> 通过 vault 直接访问同步后的文件
```

### 5.3 产品形态

- **CLI**：适合安装、脚本化、一次性同步和调试。
- **Web 控制台**：适合日常配置、运行观察和错误排查。
- **HTTP API**：服务 Web 控制台，也作为后续集成入口。
- **本地 vault**：用户真正拥有的数据出口，不依赖 Hoarder 专有格式读取文件。
- **SQLite metadata**：记录状态、审计、变更检测和错误信息。

## 6. 总体架构

```text
CLI / Web Console
  -> CLI handlers / Axum routes
  -> app services
  -> sync engine
  -> connector trait
  -> source connector implementation
  -> vault writer
  -> repository abstraction
  -> SeaORM + SQLite
```

### 6.1 分层职责

| 层 | 职责 | 主要目录 |
| --- | --- | --- |
| Interface | CLI 参数解析、HTTP request/response、Web 页面交互 | `src/cli.rs`、`src/api/`、`web/src/` |
| App services | source/job/run/settings 编排，复用业务流程 | `src/app/` |
| Sync runtime | 扫描、变更判断、文件写入、run/item/error 记录 | `src/sync/` |
| Connector | Hoarder connector trait、OpenDAL 实现、能力声明和配置校验 | `src/connectors/` |
| Persistence | SeaORM entities、repository、schema sync | `src/entity/`、`src/db/` |
| Core domain | 稳定 ID、状态枚举、snapshot、capability 等跨层类型 | `src/core/` |
| Packaging | 嵌入式前端资源和服务启动 | `src/assets.rs`、`src/server.rs` |

### 6.2 数据流

```text
SourceConnector.scan(cursor)
  -> ItemSnapshot stream
  -> Sync planner compares existing sync_item
  -> Changed/new files call SourceConnector.read(item_ref)
  -> VaultWriter writes tmp file
  -> Atomic promote to vault/{source_id}/normalized/path
  -> Repository records sync_item, sync_error, sync_run counts
```

### 6.3 控制流

```text
Manual run:
  CLI/API -> job_service::run_job -> sync engine -> repository

Scheduled run:
  scheduler tick -> due interval jobs -> job_service::run_job -> sync engine

Settings update:
  API/Web -> settings_service -> app_setting -> live runtime settings
```

## 7. 技术方案

### 7.1 后端技术栈

- Rust 2024：核心 binary 和 library。
- Tokio：异步运行时。
- Axum：本地 HTTP API 和嵌入式前端服务。
- SeaORM 2.0 entity-first：实体定义和 SQLite 访问。
- SQLite：本地 metadata database。
- OpenDAL：第一组 storage connector 后端。
- Clap：CLI。
- rust-embed：release binary 嵌入 `web/dist`。
- tracing：结构化日志。

### 7.2 前端技术栈

- Svelte 5。
- Vite 8。
- Tailwind CSS 4。
- Bun：依赖安装、检查和构建。
- lucide-svelte：图标。
- TypeScript DTO：与 API camelCase response 对齐。

### 7.3 数据库方案

核心表：

| 表 | 用途 |
| --- | --- |
| `source` | source 配置、启用状态、健康状态和最后检查时间 |
| `sync_job` | manual/interval job、状态、last run 元数据 |
| `sync_run` | 单次同步运行记录、状态、计数和时间 |
| `sync_item` | source item 到 vault item 的状态、hash、大小、etag 和同步结果 |
| `sync_error` | connector 级或 item 级错误 |
| `app_setting` | 运行时可变设置 |

当前阶段使用扁平 SQLite schema：本地资源使用 integer ID，表之间保留 `source_id`、`job_id`、`run_id` 等普通引用列，但不创建数据库 foreign key。repository 层负责必要的实时引用校验，历史 run/error/item 记录通过快照字段保持可读。

SeaORM entity registry schema sync 仍作为本地初始化机制：

1. 保留 SeaORM entities 作为类型与 repository 代码的核心模型。
2. `cargo run -- db sync` 继续作为本地 schema 初始化和开发辅助命令。
3. 显式 SQLite index 在 schema sync 后创建，匹配 job/source、run 时间、item source/path/status 和 error 查询路径。
4. schema 变更必须同步更新 entity、repository、`db_schema` 测试和相关 API/CLI 文档。
5. 发布构建需要覆盖空数据库启动 smoke test，确保新用户首次启动能完成初始化。
6. 涉及已有数据兼容性的变更在 release notes 中写清影响和处理方式。

### 7.4 Connector 方案

Connector trait 必须继续表达 Hoarder 领域语义，而不是泄漏 OpenDAL 类型：

```text
SourceConnector
  - kind()
  - validate(config)
  - scan(cursor)
  - read(item_ref)
```

`ItemSnapshot` 是 sync engine 的最小公共模型：

```text
source_id
source_path
item_type
size
etag
modified_at
content_hash
metadata_json
```

OpenDAL connector 的推进顺序：

1. `fs`：已实现，继续作为 contract regression 的基准。
2. `webdav`：优先支持 NAS、Nextcloud、坚果云等常见私有云路径。
3. `sftp`：覆盖 SSH 文件服务器和传统 NAS。
4. `s3`：覆盖对象存储、MinIO 和兼容 S3 服务。
5. NAS presets：在不破坏通用 OpenDAL 配置的前提下提供常见模板。

每个 connector 必须满足：

- 配置校验和脱敏。
- source test 可用。
- scan 支持有界内存。
- read 使用流式读取。
- 错误映射为稳定 Hoarder error code。
- API/CLI/Web 有一致配置入口。
- 集成测试覆盖成功、认证失败、路径异常和连接失败。

### 7.5 同步运行时方案

同步模型保持单向：

```text
source -> local vault
```

默认策略：

- 本地文件不自动删除。
- 源端消失的 item 标记为 `deleted_on_source`。
- item 级失败记录错误但不中止整个 run。
- connector 级失败使 run 失败。
- 通过 size、etag、modified_at 和 content_hash 判断变更。
- 文件读取和写入必须流式执行。
- job 并发和 file 并发受运行时 settings 控制。

下一步增强：

- transient error retry，带最大次数和退避策略。
- connector cursor，用于支持增量扫描。
- stale running job recovery，处理进程崩溃后 job 状态卡在 `running`。
- retention policy，清理旧 run/error/item 历史或限制记录规模。
- 性能 benchmark，覆盖大目录、多小文件、大文件和网络 connector。

### 7.6 API 方案

已有 API 保持稳定：

```text
GET    /api/health
GET    /api/openapi.json
GET    /api/sources
POST   /api/sources
POST   /api/sources/{id}/test
GET    /api/jobs
POST   /api/jobs
POST   /api/jobs/{id}/run
GET    /api/runs
GET    /api/runs/{id}
GET    /api/items
GET    /api/errors
GET    /api/settings
PATCH  /api/settings
```

API 原则：

- routes 只做提取、调用 service、映射 DTO。
- 错误统一使用结构化 JSON shape。
- 不向响应暴露内部数据库、IO stack trace 或 secret。
- OpenAPI 必须随 DTO 和 route 更新。
- 新增 query/filter 先明确索引和分页策略，避免未来列表接口膨胀。

需要补充：

- list endpoints 的分页和排序。
- source/job enable/disable API。
- job 删除或归档 API。
- run retention/prune API。
- connector-specific config schema metadata，便于 Web 表单自动渲染。

### 7.7 CLI 方案

CLI 保持显式、脚本友好：

```text
hoarder serve
hoarder db sync
hoarder source list
hoarder source add --name docs --service fs --root ./docs
hoarder source test --id 1
hoarder job list
hoarder job add --source-id 1 --name docs --interval 300
hoarder sync run --job-id 1
hoarder sync status
```

下一步：

- 增加 `--json` 输出模式，便于自动化集成。
- 增加 `source disable/enable`、`job disable/enable`。
- 增加 connector-specific typed flags。
- 保持 human-readable table 作为默认输出。
- CLI 与 API 继续复用 app services，不复制业务逻辑。

### 7.8 Web 控制台方案

Web 控制台定位为本地运维界面，而不是营销页面。页面继续保持紧凑、工具化、可扫描：

- Overview：整体健康、最近 runs、错误摘要、job 状态。
- Sources：创建、测试、启用状态、健康状态、配置脱敏展示。
- Jobs：创建 manual/interval job、触发 run、查看 next run。
- Runs：run 列表、详情、item 和 error 过滤。
- Settings：运行时设置编辑，启动期配置只读展示。

下一步：

- 可访问性检查：键盘导航、focus 状态、表单 label、状态文本、屏幕阅读器。
- 浏览器截图回归：关键页面 desktop/mobile 截图。
- 错误态和空态完善。
- connector-specific 表单模板。
- 大列表分页、筛选和局部刷新。

### 7.9 安全方案

默认安全策略：

- 默认监听 `127.0.0.1`。
- connector secret 在日志和 API response 中脱敏。
- 所有外部输入在边界校验。
- SQLite 操作通过 SeaORM。
- vault path 写入前规范化。
- 拒绝绝对路径和 `..` 路径穿越。
- 保护 `.hoarder` 保留目录。
- 临时文件写入后原子替换。
- 默认不删除本地 vault 文件。

未来如进入远程多用户模式，必须先补：

- 认证和 session/token 策略。
- 授权模型。
- CSRF/CORS 策略调整。
- secret at rest 加密。
- audit log。
- 多进程或多实例调度锁。

### 7.10 可观测性方案

已有：

- Axum structured request logging。
- run/item/error 持久化。
- source health。
- job last run 信息。

下一步：

- 为 connector error 建立稳定错误码表。
- 增加 run duration、bytes/sec、items/sec 指标。
- 增加最近错误摘要。
- 增加 debug bundle 导出，便于用户提交问题时脱敏上传。
- 长时间运行时记录 scheduler tick 和 skipped reason。

### 7.11 测试和质量方案

合并前命令：

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cd web
bun run verify
```

发布前命令：

```bash
cd web
bun run build
cd ..
cargo build --release
```

测试分层：

- Unit tests：纯函数、DTO、path normalization、config validation。
- Integration tests：repository、API routes、CLI commands、sync engine。
- Connector tests：fs 基准、WebDAV/SFTP/S3 使用可控测试服务或 mock。
- E2E tests：本地 source -> vault 完整流程。
- Browser tests：关键 Web 页面截图、console error、表单路径。
- Soak tests：长时间 interval job、大目录、多小文件和网络不稳定场景。

### 7.12 发布方案

发布目标：

- 单 Rust binary。
- 嵌入当前 `web/dist`。
- macOS、Linux、Windows release artifacts。
- 版本号遵循 SemVer。
- release notes 包含新能力、破坏性变更、数据兼容性说明和已知问题。

发布流程：

1. CI 跑格式、lint、test、frontend verify。
2. 构建 `web/dist`。
3. 构建 release binary。
4. 运行 smoke test。
5. 打 tag。
6. 生成平台 artifacts。
7. 发布 release notes。

## 8. 实施计划

### Phase 0：当前 MVP 基线

目标：明确已完成能力，冻结当前可运行主路径。

验收：

- filesystem source 可以创建、测试、同步。
- manual 和 interval job 可运行。
- Web 控制台核心页面可访问。
- API 和 CLI 主路径可用。
- 基础测试门禁通过。

状态：基本完成。

### Phase 1：稳定化和可发布基础

目标：让项目从“本地可用 MVP”进入“可重复发布的产品基线”。

任务：

- 维护 CI workflow。
- 维护 release build workflow。
- 维护 README/development 发布说明。
- 增加 Web 可访问性检查。
- 增加浏览器截图回归。
- 增加 stale running job recovery 设计和实现。
- 增加 database retention policy 设计。

验收：

- CI 对每个 PR 自动运行 Rust 和 Web 门禁。
- release workflow 能产出至少一个平台 binary。
- 新库从空数据库启动可完成 schema 初始化。
- 旧数据库升级有测试覆盖。
- Web 关键页面无明显键盘导航阻断。

### Phase 2：连接器扩展

目标：覆盖真实私有存储和对象存储场景。

任务：

- 验证 OpenDAL WebDAV operator against 可控测试服务。
- 验证 OpenDAL SFTP operator against 可控测试服务。
- 验证 OpenDAL S3 operator against 可控测试服务。
- 增加 connector-specific typed config。
- 增加 NAS presets。
- 增加 connector integration tests。
- Web source form 支持不同 service 的字段模板。
- CLI source add 支持 service-specific flags。

验收：

- WebDAV/SFTP/S3 均能 create、test、sync。
- secret 在 API 和日志中脱敏。
- 网络连接失败有可理解错误。
- 大目录扫描保持有界内存。
- UI 能展示不同 connector 的配置要求。

### Phase 3：同步可靠性增强

目标：降低网络 connector 的失败成本，提升长期运行稳定性。

任务：

- transient error retry。
- exponential backoff。
- connector cursor contract。
- 增量扫描支持。
- run cancellation 设计。
- scheduler skipped reason logging。
- 扩展长时间运行 soak tests。
- 扩展性能 benchmark。

验收：

- 临时网络错误可按策略重试。
- connector 支持 cursor 时可避免每次全量扫描。
- 大目录和大文件同步有 benchmark 基线。
- interval jobs 长时间运行无明显状态漂移。

### Phase 4：检索和组织能力

目标：从同步平台升级为本地数据工作台。

任务：

- 全文搜索方案选型。
- 建立索引任务和索引状态。
- Web 搜索页面。
- 标签或集合模型。
- 跨 source 去重的 hash/index 基础。
- 通知策略设计。

验收：

- 用户可按关键词搜索已同步文件元数据或文本内容。
- 搜索结果可定位到 vault 文件。
- 标签/集合不会破坏 source 原始路径。
- 索引失败可审计、可重试。

### Phase 5：高级能力探索

目标：在已有可靠单向模型上探索更高风险能力。

候选：

- Notion connector。
- 飞书 connector。
- 自动本地删除策略。
- 远程多用户部署。
- 第三方插件 ABI。

进入条件：

- Phase 1-3 稳定通过。
- 有明确用户需求和安全边界。
- 对数据破坏性操作有 dry-run、审计和回滚策略。

## 9. Roadmap

时间以 2026-05-27 为基准，具体排期可按团队容量调整。

| 时间窗口 | 里程碑 | 主要交付 |
| --- | --- | --- |
| 第 1-2 周 | M1：发布基线 | CI、release build、文档更新、基础 browser/a11y 检查 |
| 第 3-4 周 | M2：WebDAV/SFTP | WebDAV connector、SFTP connector、配置表单、CLI typed flags、集成测试 |
| 第 5-6 周 | M3：S3 和 NAS 场景 | S3 connector、MinIO/S3-compatible 验证、NAS presets、错误码表 |
| 第 7-8 周 | M4：可靠性增强 | retry、backoff、stale job recovery、retention policy、soak test |
| 第 9-12 周 | M5：搜索 Alpha | 全文搜索技术方案、索引模型、搜索 API、Web 搜索页首版 |
| 第 13-16 周 | M6：组织能力 | 标签/集合、跨 source 去重基础、通知设计和首个实现 |
| 16 周以后 | M7：高级 connector 和远程模式评估 | Notion/飞书原型、多用户架构 ADR |

优先级：

1. M1 必须优先，解决可发布和可维护问题。
2. M2-M3 直接提升产品适用场景，是最重要的产品扩展。
3. M4 是网络 connector 进入真实使用后的稳定性保障。
4. M5-M6 将产品从同步工具扩展为数据工作台。
5. M7 不应在本地单向模型稳定前提前投入大规模实现。

## 10. 风险与应对

| 风险 | 影响 | 应对 |
| --- | --- | --- |
| 网络 connector 行为差异大 | 同步失败率高、错误难排查 | 为每个 connector 建立独立 test matrix 和稳定错误码 |
| schema 初始化行为不清晰 | 新用户首次启动或版本变化后失败 | 用 `db_schema` 测试和启动 smoke test 覆盖空数据库与已有数据场景 |
| Web UI mock fallback 掩盖真实 API 问题 | UI 看似正常但 live path 失败 | mock 仅用于 API 不可用预览，关键路径用集成或浏览器测试覆盖 |
| 自动删除过早引入 | 数据破坏风险高 | 保持后置，先实现 dry-run、审计和回滚策略 |
| 单进程 scheduler 状态异常 | job 卡住或重复运行 | 增加 running guard、stale recovery 和 scheduler 日志 |
| Release artifacts 缺少 smoke test | 用户下载后无法启动 | 发布流水线加入 `serve` 启动、health check 和 embedded asset check |
| 搜索索引膨胀 | 本地磁盘占用不可控 | 索引大小指标、清理策略、可关闭索引和按 source 控制 |

## 11. 验收标准

### 11.1 阶段验收

每个阶段完成时必须满足：

- 文档更新。
- 相关测试新增或更新。
- `cargo fmt --check` 通过。
- `cargo clippy --all-targets --all-features -- -D warnings` 通过。
- `cargo test` 通过。
- 如涉及 `web/src/`，`cd web && bun run verify` 通过。
- 如涉及发布，`cd web && bun run build && cd .. && cargo build --release` 通过。

### 11.2 产品验收

- 用户能完成 source -> job -> run -> inspect -> vault access 的完整闭环。
- connector secret 不出现在 API response、日志或错误详情中。
- 同步失败能定位到 connector、source path、run 和错误码。
- 本地 vault 文件可直接阅读，不依赖 Hoarder 专有查看器。
- 默认配置不暴露远程端口，不做破坏性本地删除。

## 12. 决策记录

当前继续沿用以下决策：

- 本地优先，默认监听 `127.0.0.1`。
- 第一阶段坚持单向同步。
- OpenDAL 是第一组 connector 后端，但内部 contract 不暴露 OpenDAL 类型。
- SQLite 是本地 metadata database。
- SeaORM entities 保持代码模型来源，当前阶段继续使用 schema sync 初始化和演进本地 SQLite schema。
- CLI、API、scheduler 和 Web 共享 app services。
- 前端保持工具型控制台，不做 landing page。
- release 目标是嵌入式前端的单二进制。

后续需要补充 ADR：

- ADR：SQLite schema sync 的使用边界。
- ADR：connector cursor contract。
- ADR：全文搜索索引方案。
- ADR：远程多用户模式是否进入项目范围。
- ADR：自动删除策略的安全边界。
