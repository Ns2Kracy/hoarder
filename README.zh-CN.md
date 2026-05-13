# Hoarder

[English](README.md)

Hoarder 是一个本地优先的数据聚合和单向同步平台。它用于连接外部数据源，把内容写入可读的本地 vault，并把同步状态记录到 SQLite 中，方便通过本地 CLI、API 或 Web 控制台查看运行记录、排查问题和审计同步结果。

当前版本优先打好本地运行基础：Rust、Axum、SeaORM 2.0 entity-first、SQLite、OpenDAL、Svelte、Tailwind CSS、Bun，以及把前端资源嵌入 Rust 单二进制文件的发布路径。

## 项目亮点

- 本地优先：数据写入你自己的文件系统，元数据存储在本地 SQLite。
- 单向同步：数据从 source 写入 vault，不会把本地文件反向推回数据源。
- 可读的 vault 结构：同步后的文件位于 `vault/{source_id}/normalized/source/path`。
- 连接器抽象清晰：同步核心依赖 Hoarder 自己的 trait，不直接依赖 OpenDAL 或具体厂商 API。
- OpenDAL 是第一组连接器能力：文件系统同步已经可用；`fs`、`webdav`、`sftp`、`s3` 的配置模型已经存在。
- 安全写入：文件先流式写入临时路径，再原子替换到最终 vault 路径。
- 默认不删除本地文件：源端消失的文件会标记为 `deleted_on_source`，但本地 vault 文件会保留。
- 结构化运行历史：run、item、error、计数、hash、时间戳都会持久化。
- 单二进制发布：Rust release binary 会嵌入 `web/dist` 前端资源。
- 严格质量门禁：`Cargo.toml` 中已开启 Rust warnings 和严格 Clippy deny 规则。

## 快速开始

前置要求：

- Rust 2024 toolchain
- Bun

先构建 Web UI，再启动本地服务：

```bash
cd web
bun install
bun run build
cd ..
cargo run -- serve
```

打开：

```text
http://127.0.0.1:4761
```

使用自定义配置：

```json
{
  "databasePath": "./hoarder.db",
  "vaultPath": "./vault",
  "listenAddr": "127.0.0.1:4761",
  "jobConcurrency": 1,
  "fileConcurrency": 4
}
```

```bash
cargo run -- --config ./hoarder.config.json serve
```

## 命令

| 命令 | 状态 | 说明 |
| --- | --- | --- |
| `cargo run -- serve` | [x] | 启动 Axum API 和嵌入式 Web 控制台。 |
| `cargo run -- serve --addr 127.0.0.1:4762` | [x] | 覆盖监听地址。 |
| `cargo run -- --config ./hoarder.config.json serve` | [x] | 启动前读取 JSON 配置。 |
| `cargo run -- db sync` | [x] | 根据 SeaORM entities 同步 SQLite schema。 |
| `cargo run -- source list` | [ ] | CLI source 列表处理逻辑。 |
| `cargo run -- source add ...` | [ ] | CLI source 创建处理逻辑。 |
| `cargo run -- source test ...` | [ ] | CLI source 校验处理逻辑。 |
| `cargo run -- sync run ...` | [ ] | CLI 一次性同步处理逻辑。 |
| `cargo run -- sync status` | [ ] | CLI 同步状态处理逻辑。 |

## 功能清单

### 核心平台

- [x] Rust 2024 后端
- [x] Tokio async runtime
- [x] Axum 本地 HTTP API
- [x] 本地优先默认监听地址：`127.0.0.1:4761`
- [x] JSON 配置文件支持
- [x] 基于 Clap 的 CLI 解析
- [x] UUID v4 标识符
- [x] 标准库文件系统路径
- [x] `Cargo.toml` 中启用严格 Rust 和 Clippy lint
- [x] Release profile 启用 LTO 和符号裁剪
- [ ] serve 模式内置后台调度器
- [ ] 运行时设置持久化和修改
- [ ] 远程多用户部署模式
- [ ] 认证和授权

### 持久化

- [x] SQLite 元数据数据库
- [x] SeaORM 2.0 entity-first 模型定义
- [x] Entity registry schema sync
- [x] `source` 记录
- [x] `sync_job` 记录
- [x] `sync_run` 记录
- [x] `sync_item` 记录
- [x] `sync_error` 记录
- [x] 面向同步引擎测试的 repository 抽象
- [x] SeaORM repository 实现
- [ ] 配置文件之外的持久化 app settings
- [ ] 显式 schema migration
- [ ] 数据库清理或保留策略

### 连接器

- [x] Connector trait 边界
- [x] Connector capability 模型
- [x] `opendal` connector kind
- [x] 在领域类型中预留 `notion` 和 `feishu` connector kind
- [x] OpenDAL `fs` 服务配置校验
- [x] OpenDAL `webdav` 服务配置校验
- [x] OpenDAL `sftp` 服务配置校验
- [x] OpenDAL `s3` 服务配置校验
- [x] Connector options 敏感信息脱敏
- [x] OpenDAL 文件系统扫描
- [x] OpenDAL 文件系统文件读取
- [x] 目录和文件元数据映射到 Hoarder snapshot
- [ ] OpenDAL WebDAV operator 实现
- [ ] OpenDAL SFTP operator 实现
- [ ] OpenDAL S3 operator 实现
- [ ] NAS 专用预设或模板
- [ ] Notion connector 实现
- [ ] 飞书 connector 实现
- [ ] Connector pagination 或 incremental cursor 支持
- [ ] 第三方编译插件 ABI

### 同步运行时

- [x] 从 source 到本地 vault 的单向同步
- [x] Source path 规范化
- [x] 拒绝绝对路径
- [x] 拒绝路径穿越
- [x] 保护保留目录 `.hoarder`
- [x] `vault/{source_id}/...` 可读布局
- [x] 临时文件写入
- [x] 原子替换到最终 vault 路径
- [x] SHA-256 内容 hash 记录
- [x] 新 item 检测
- [x] 按 item type、ETag、size、modified time 和 hash 检测变更
- [x] 未变化 item 跳过
- [x] 源端删除检测
- [x] 源端消失的 item 标记为 `deleted_on_source`
- [x] 源端 item 消失时保留本地 vault 文件
- [x] 单个 item 失败不会导致整个 run 失败
- [x] Connector 级失败会让 run 失败
- [x] Run summary 记录 processed、synced、skipped、failed 和 byte count
- [ ] 有界并发文件同步
- [ ] Job 级并发控制
- [ ] 定时同步任务
- [ ] 从 connector cursor 恢复
- [ ] 临时性 connector 错误重试策略
- [ ] 冲突处理
- [ ] 双向同步
- [ ] 自动本地删除策略

### API

- [x] `GET /api/health`
- [x] `GET /api/sources`
- [x] `POST /api/sources`
- [x] `GET /api/jobs`
- [x] `POST /api/jobs/{id}/run`
- [x] `GET /api/runs`
- [x] `GET /api/items`
- [x] `GET /api/errors`
- [x] `GET /api/settings`
- [x] 稳定的结构化错误响应
- [x] API 错误隐藏内部数据库和 IO 细节
- [x] 未匹配的 `/api/*` 路由保持 JSON 错误形态
- [ ] `POST /api/sources/{id}/test`
- [ ] `POST /api/jobs`
- [ ] `GET /api/runs/{id}`
- [ ] 按 source 或 status 过滤 item 列表
- [ ] `PATCH /api/settings`
- [ ] OpenAPI specification

### Web 控制台

- [x] Svelte 5 前端
- [x] Vite 8 构建
- [x] 通过 `@tailwindcss/vite` 使用 Tailwind CSS 4
- [x] 基于 Bun 的前端安装、检查和构建脚本
- [x] 由 Axum 提供嵌入式生产前端资源
- [x] 响应式侧边栏布局
- [x] Overview 页面
- [x] Sources 页面
- [x] OpenDAL 风格 source 配置创建表单
- [x] Jobs 页面
- [x] Runs 页面
- [x] Settings 页面
- [x] 状态徽标和紧凑型运营表格
- [x] API client 带 mock fallback，方便本地 API 不可用时预览
- [ ] 所有展示控件完全接入 live API
- [ ] Source test action 接入 API route
- [ ] Settings save 接入 API route
- [ ] Run detail endpoint 集成
- [ ] 键盘和屏幕阅读器可访问性检查
- [ ] 浏览器截图回归检查

### 打包和质量

- [x] 单个 Rust binary 嵌入 `web/dist` 前端资源
- [x] `cargo fmt --check`
- [x] 严格 `cargo clippy --all-targets --all-features`
- [x] `cargo test`
- [x] `bun run verify`
- [x] `cargo build --release`
- [x] 本地文件系统端到端同步测试
- [x] 静态资源 fallback 测试
- [x] API route 测试
- [x] Connector contract 测试
- [x] Vault writer 安全测试
- [ ] CI workflow
- [ ] macOS、Linux、Windows release artifacts
- [ ] Installer 或包管理器分发
- [ ] 性能 benchmark
- [ ] 长时间运行 soak test

### 产品路线图

- [ ] 全文搜索
- [ ] 双向同步
- [ ] 自动本地删除策略
- [ ] 跨 source 去重
- [ ] 标签或集合
- [ ] 通知
- [ ] Source 定义导入/导出

## 架构

```text
CLI / Web UI
  -> command handlers / Axum API
  -> sync engine
  -> connector trait
  -> source connector
  -> vault writer
  -> SeaORM repository
  -> SQLite
```

关键边界：

- `src/core`：跨层共享的稳定领域类型。
- `src/connectors`：连接器 trait 和 OpenDAL-backed 实现。
- `src/sync`：planner、engine、repository trait 和 vault writer。
- `src/db`：SeaORM repository 和 schema sync。
- `src/api`：DTO、routes、state traits 和错误映射。
- `src/server.rs`：Axum server 组装和数据库驱动的 API wiring。
- `web`：Svelte 管理控制台。

## 本地 Vault 布局

```text
vault/
  {source_id}/
    normalized/source/path.ext
  .hoarder/
    tmp/
```

写入前会规范化 source path。Hoarder 会拒绝绝对路径、路径穿越、NUL 字节、Windows drive prefix，以及任何写入保留目录 `.hoarder` 的尝试。

## 开发

安装前端依赖：

```bash
cd web
bun install
```

运行前端验证：

```bash
cd web
bun run verify
```

运行后端验证：

```bash
cargo fmt --check
cargo clippy --all-targets --all-features --message-format=short
cargo test
```

构建打包后的 release binary：

```bash
cd web
bun run build
cd ..
cargo build --release
```

## 当前状态

Hoarder 目前是早期本地优先 MVP。后端已经可以提供 Web 控制台、同步 SQLite schema、通过 API 创建 source、通过 API 运行数据库中的 job，并通过本地文件系统端到端同步测试。下一步最高价值的工作是补齐剩余 CLI handler、增加 source test/settings API routes，并实现 filesystem 之外的更多 OpenDAL 服务。
