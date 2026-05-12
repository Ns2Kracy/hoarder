export type PageId = "overview" | "sources" | "jobs" | "runs" | "settings";

export type DataOrigin = "api" | "mock";

export type ConnectorKind = "opendal";

export type OpenDalServiceKind = "fs" | "s3" | "webdav" | "sftp";

export type SourceHealth = "healthy" | "warning" | "failed" | "untested" | "disabled";

export type JobStatus = "scheduled" | "running" | "paused" | "failed";

export type RunStatus = "running" | "completed" | "failed" | "cancelled";

export type ItemSyncStatus = "pending" | "synced" | "skipped" | "failed" | "deleted_on_source";

export type LogLevel = "trace" | "debug" | "info" | "warn" | "error";

export interface ApiErrorBody {
  error: {
    code: string;
    message: string;
    details?: Record<string, unknown>;
  };
}

export interface FrontendApiError {
  code: string;
  message: string;
  details?: Record<string, unknown>;
  status?: number;
}

export interface RedactedConfig {
  service: OpenDalServiceKind;
  root?: string;
  endpoint?: string;
  bucket?: string;
  region?: string;
  username?: string;
  access_key_id?: string;
  secret_access_key?: string;
  token?: string;
  [key: string]: unknown;
}

export interface SourceDto {
  id: string;
  name: string;
  connectorKind: ConnectorKind;
  serviceKind: OpenDalServiceKind;
  enabled: boolean;
  config: RedactedConfig;
  health: SourceHealth;
  itemCount: number;
  lastCheckedAt?: string;
  lastRunAt?: string;
  lastError?: string;
}

export interface SourceFormInput {
  name: string;
  serviceKind: OpenDalServiceKind;
  enabled: boolean;
  config: {
    root?: string;
    endpoint?: string;
    bucket?: string;
    region?: string;
    username?: string;
    accessKeyId?: string;
    secretAccessKey?: string;
    token?: string;
  };
}

export interface SyncJobDto {
  id: string;
  sourceId: string;
  sourceName: string;
  schedule: string;
  enabled: boolean;
  status: JobStatus;
  nextRunAt?: string;
  lastRunAt?: string;
  lastRunStatus?: RunStatus;
}

export interface RunCounts {
  processed: number;
  synced: number;
  skipped: number;
  failed: number;
  deleted: number;
}

export interface SyncErrorDto {
  id: string;
  runId: string;
  sourcePath?: string;
  code: string;
  message: string;
  details?: Record<string, unknown>;
  createdAt: string;
}

export interface SyncRunDto {
  id: string;
  jobId: string;
  sourceId: string;
  sourceName: string;
  status: RunStatus;
  startedAt: string;
  finishedAt?: string;
  durationMs?: number;
  counts: RunCounts;
  errors: SyncErrorDto[];
}

export interface SettingsDto {
  vaultPath: string;
  databasePath: string;
  listenAddress: string;
  jobConcurrency: number;
  fileConcurrency: number;
  logLevel: LogLevel;
}

export type SettingsUpdate = SettingsDto;

export interface Loadable<T> {
  status: "idle" | "loading" | "ready" | "empty" | "error";
  data: T;
  origin: DataOrigin;
  error?: FrontendApiError;
  updatedAt?: string;
}

export interface ConsoleSummary {
  sourceCount: number;
  enabledSourceCount: number;
  activeJobCount: number;
  runningJobCount: number;
  failedItemCount: number;
  vaultSizeLabel: string;
  lastRun?: SyncRunDto;
}

export interface ApiData<T> {
  data: T;
  origin: DataOrigin;
  error?: FrontendApiError;
}
