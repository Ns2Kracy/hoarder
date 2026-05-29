# Hoarder PRD

日期：2026-05-29
状态：当前 MVP 产品定义与下一阶段产品方向
读者：产品负责人、工程负责人、后端/前端工程师、测试与发布负责人

## 1. 产品定位

Hoarder 是一个本地优先的数据聚合与单向同步平台，也是在 AI 盛行背景下为 RAG 和个人/团队知识库准备的本地数据底座。它把外部 source 中的文件或文档同步到用户可直接读取的本地 vault，并用 SQLite 记录 source、job、run、item、error 等元数据，让用户可以通过 CLI、HTTP API 和 Web 控制台完成配置、运行、审计和排障。

当前产品不是远程 SaaS，也不是双向网盘同步器。它优先服务个人、开发者和小团队在本地或内网环境中安全汇聚数据的需求。

## 2. 用户与问题

| 用户 | 典型问题 | Hoarder 提供的价值 |
| --- | --- | --- |
| 个人数据归档用户 | 文件散落在本地目录、NAS、WebDAV、S3 或 SFTP 中，缺少统一归档出口 | 把多 source 内容同步到可读本地 vault，保留原始路径结构 |
| 开发者/技术用户 | 希望用脚本化方式管理 source、job 和一次性同步 | CLI 与 HTTP API 暴露同一套业务能力，便于自动化 |
| 小团队工具维护者 | 需要在内网机器上查看 source 健康、job 状态、错误和历史 run | 本地 Web 控制台提供运营视图与排障入口 |

核心问题：用户需要“可拥有、可审计、可恢复”的数据汇聚工具，而不是把数据再次托管到远程服务中。对 RAG 场景来说，Hoarder 首先解决“可信数据在哪里、如何持续同步、如何审计来源和更新”的问题，再为后续解析、切分、索引、向量检索和 agent 使用提供稳定输入。

## 3. 产品目标

1. **本地优先**：默认监听 `127.0.0.1`，数据写入本地文件系统，元数据写入本地 SQLite。
2. **单向安全同步**：只执行 `source -> local vault`，不把本地变更写回 source。
3. **可读输出**：vault 以 `vault/{source_id}/normalized/source/path` 存储文件，不依赖 Hoarder 才能读取内容。
4. **可审计运行**：每次 run 都记录状态、计数、耗时、item 结果、错误和 hash。
5. **多入口一致**：CLI、API、Web 控制台复用同一套 app service 和 sync engine。
6. **连接器可扩展**：同步核心依赖 Hoarder connector trait，不直接耦合具体存储厂商。
7. **RAG-ready 知识库底座**：先聚合并保留 source provenance、路径、hash、时间戳和同步状态，后续在此基础上建立内容解析、chunk、embedding、全文/向量检索和 RAG 查询层。

## 4. 非目标

当前阶段明确不做：

- 远程多用户部署、认证、授权和租户隔离。
- 双向同步、冲突合并和把本地文件推回 source。
- 默认自动删除本地 vault 文件。
- 全文搜索、标签、集合和通知的完整产品化版本。
- 内置 embedding model、vector database、LLM gateway 或完整 RAG query runtime。
- 第三方编译插件的动态加载、沙箱执行和远程分发。
- Notion、飞书等应用 connector 的完整富文本渲染、附件导出和双向写回。

这些能力可以进入后续 roadmap，但必须建立在稳定的本地单用户同步模型之上。

## 5. 当前产品范围

### 5.1 核心用户旅程

```mermaid
flowchart LR
    A[安装 Hoarder] --> B[启动本地服务]
    B --> C[创建 source]
    C --> D[测试 source]
    D --> E[创建 manual 或 interval job]
    E --> F[手动触发第一次 run]
    F --> G[查看 run detail、items、errors]
    G --> H[serve 模式定期同步]
    H --> I[直接访问本地 vault 文件]
```

### 5.2 功能需求

| ID | 功能 | 当前状态 | 验收标准 |
| --- | --- | --- | --- |
| P-01 | Source 创建、编辑、列表 | 已实现 | 用户可配置 OpenDAL source，敏感字段返回时脱敏 |
| P-02 | Source 连接测试 | 已实现 | 测试结果写入 source 健康状态和最后检查时间 |
| P-03 | Job 创建、编辑、列表 | 已实现 | 支持 manual 与固定 interval schedule，禁用 job 不会运行 |
| P-04 | 手动运行 job | 已实现 | CLI/API/Web 可触发同一条 `job_service::run_job` 路径 |
| P-05 | serve 模式调度 | 已实现 | 调度器按固定 tick 启动到期 interval jobs，并受 `job_concurrency` 限制 |
| P-06 | Run 历史与详情 | 已实现 | 可查看状态、开始/结束时间、耗时、计数和关联错误 |
| P-07 | Item 与 error 查询 | 已实现 | 支持按 source、run、status 过滤 item，按 source 或 run 过滤 error |
| P-08 | Runtime settings | 已实现 | `job_concurrency`、`file_concurrency`、`log_level` 可持久化修改 |
| P-09 | 本地 vault 写入 | 已实现 | 流式写入临时文件后原子替换，拒绝危险路径 |
| P-10 | 单二进制发布路径 | 已实现 | `web/dist` 被嵌入 Rust release binary |
| P-11 | CI 与 release artifacts | 未实现 | macOS、Linux、Windows 构建与测试可在 CI 中复现 |
| P-12 | 可访问性与截图回归 | 未实现 | Web 控制台通过基础键盘/屏幕阅读器检查并保留浏览器回归证据 |
| P-13 | RAG-ready knowledge base | 规划中 | 对同步内容建立可追溯的解析、chunk、索引和检索链路 |

### 5.3 Connector 范围

| Connector family | Service | 当前状态 | 产品说明 |
| --- | --- | --- | --- |
| OpenDAL | `fs` | 已实现 | 本地目录同步，是 MVP 基准路径 |
| OpenDAL | `webdav` | 已实现 operator wiring | 面向 NAS、Nextcloud、WebDAV 私有云 |
| OpenDAL | `sftp` | 已实现 operator wiring | 面向 SSH 文件服务器、传统 NAS |
| OpenDAL | `s3` | 已实现 operator wiring | 面向 S3/MinIO/兼容对象存储 |
| App connectors | `notion`、`feishu` | 已实现基础扫描与读取 | 以 `virtual_document` 写入 vault，保留 source provenance，支持分页 cursor |
| Plugin connectors | 第三方编译插件 | 已实现 ABI 契约 | 当前提供 manifest/ABI 校验与配置形状，动态加载作为后续能力 |

### 5.4 AI/RAG 知识库聚合定位

Hoarder 在 AI/RAG 链路中的定位是 **knowledge ingestion and provenance layer**。它不急于把 LLM、embedding、vector store 都塞进 MVP，而是先把最容易决定 RAG 质量的数据准备层做好。

| RAG 阶段 | Hoarder 当前/规划职责 | 当前状态 |
| --- | --- | --- |
| Source ingestion | 连接 filesystem、WebDAV、SFTP、S3 等 source，持续同步到本地 vault | 已实现基础路径 |
| Provenance | 记录 source、source_path、local_path、hash、mtime、etag、run、error | 已实现 |
| Change tracking | 判断新增、变更、跳过、失败、源端删除 | 已实现 |
| Content normalization | 将应用类 source 导出为 markdown/json/html/attachments | 规划中 |
| Parsing and chunking | 解析 vault 文件、生成 chunk，保留 source provenance | 规划中 |
| Search index | 全文搜索、metadata filter、向量索引 | 规划中 |
| RAG serving | 给本地 agent/LLM 提供检索 API、引用和上下文包 | 规划中 |

产品原则：RAG 能力必须建立在可审计数据链路上。每个用于回答问题的 chunk 都应能追溯到 source、原始路径、同步 run、hash 和本地文件位置，避免“知识库里有内容但不知道从哪里来、什么时候更新、是否同步失败”的问题。

## 6. 用户体验要求

### 6.1 Web 控制台信息架构

| 页面 | 用户目标 | 关键内容 |
| --- | --- | --- |
| Overview | 快速判断系统是否健康 | source/job/run 摘要、失败 item、vault size、最近 run、source health |
| Sources | 管理数据来源 | source 表单、服务类型、位置、健康状态、测试、编辑 |
| Jobs | 管理同步计划 | job 表单、schedule、last/next run、运行按钮、编辑 |
| Runs | 审计同步结果 | run 列表、run detail、item 列表、error 列表 |
| Settings | 调整运行参数 | 并发、日志级别、只读启动配置展示 |

### 6.2 CLI 体验

CLI 面向自动化与本地排障，应保持显式 flag 风格，避免要求用户手写大段 JSON：

```text
hoarder source add --name docs --service fs --root ./docs
hoarder source test --id 1
hoarder job add --source-id 1 --name docs --interval 300
hoarder sync run --job-id 1
hoarder sync status
```

### 6.3 错误体验

- 用户可理解的错误应返回稳定 code 和 message。
- 数据库与 IO 内部细节不暴露给 API 调用方。
- Connector 错误归类为 connector failure，item 级失败应写入 run 详情和 error 列表。
- Source 配置中的 secret 字段在 API 和 UI 中必须脱敏。

## 7. 技术需求

| 维度 | 要求 |
| --- | --- |
| 安全 | 默认 loopback；CORS 仅允许本地开发端口；拒绝绝对路径、路径穿越、NUL、Windows drive prefix 和 `.hoarder` 写入 |
| 可靠性 | item 级失败不中断整个 run；connector 级失败使 run 失败；每次 run 都落库 |
| 性能 | scan 与 read 使用 stream；文件同步受 `file_concurrency` 限制；job 启动受 `job_concurrency` 限制 |
| 可扩展性 | connector trait 输出 Hoarder 领域模型；sync engine 不依赖 OpenDAL 类型 |
| 可维护性 | route 只做提取和响应映射；业务编排放在 `src/app/`；持久化放在 repository |
| 可发布性 | 前端先构建，Rust binary 嵌入 `web/dist`；release profile 开启 LTO 和 strip |
| 可验证性 | Rust 使用 `cargo fmt --check`、strict clippy、`cargo test`；前端使用 `bun run verify` |

## 8. 成功指标

| 指标 | 目标 |
| --- | --- |
| 首次可用时间 | 新用户 10 分钟内完成 source 创建、job 创建、首次 run 和 run 查看 |
| 数据安全 | 危险 source path 被拒绝；源端删除不会触发本地文件删除 |
| 审计完整性 | 每个 run 都可查 source/job 快照、计数、状态、错误和时间 |
| 发布可复现 | CI 中稳定通过 Rust、Web、release build 验证 |
| Connector 覆盖 | `fs`、`webdav`、`sftp`、`s3` 均具备配置、测试和同步路径 |

## 9. Roadmap

### Now：MVP 稳定化

- 增加 CI workflow 和 release artifact。
- 补齐 Web 控制台可访问性与浏览器截图回归。
- 增加长时间运行 soak test 和基础性能 benchmark。
- 为 WebDAV/SFTP/S3 增加更贴近真实服务的 integration tests。

### Next：连接器与运行时增强

- Connector cursor 与增量扫描。
- 临时性 connector 错误重试策略。
- NAS 场景预设和 typed connector config 表单。
- 数据库清理与保留策略。
- 内容解析 pipeline：文件类型识别、文本抽取、chunk、metadata/provenance 绑定。

### Later：知识库能力

- 全文搜索。
- 向量索引、hybrid search、RAG 检索 API 和引用生成。
- 标签、集合、跨 source 去重。
- 通知与导入/导出 source 定义。
- Notion、飞书等应用 connector。

## 10. 开放问题

1. WebDAV/SFTP/S3 的首批真实兼容目标是什么：NAS、Nextcloud、MinIO、AWS S3，还是内网自建服务优先？
2. 全文搜索应先索引 vault 文件内容，还是只索引 metadata 与文件名？
3. 数据库保留策略应默认保留全部 run，还是提供按时间/数量的手动清理？
4. 后续是否需要多 profile 或多个 vault root，还是保持单实例单 vault？
5. RAG 索引应内置 SQLite FTS/vector extension，还是先提供可插拔 indexer，把向量库留给外部服务？
