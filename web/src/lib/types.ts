export type PageId = "overview" | "sources" | "jobs" | "runs" | "settings";

export type DataOrigin = "api" | "mock";

export type LocalId = number;

export type ConnectorKind = "opendal";

export type OpenDalServiceKind = "fs" | "s3" | "webdav" | "sftp";

export type SourceHealth = "healthy" | "warning" | "failed" | "untested" | "disabled";

export type JobScheduleKind = "manual" | "interval";

export type JobSchedule =
  | {
      kind: "manual";
    }
  | {
      kind: "interval";
      intervalSeconds: number;
    };

export type JobStatus = "idle" | "running" | "paused" | "failed";

export type RunStatus =
  | "running"
  | "completed"
  | "completed_with_failures"
  | "failed"
  | "cancelled";

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
  private_key?: string;
  [key: string]: unknown;
}

export interface SourceDto {
  id: LocalId;
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
    privateKey?: string;
  };
}

export interface SourceTemplate {
  id: string;
  label: string;
  description: string;
  serviceKind: OpenDalServiceKind;
  defaultConfig: Partial<SourceFormInput["config"]>;
}

export interface SyncJobDto {
  id: LocalId;
  sourceId: LocalId;
  sourceName: string;
  name: string;
  schedule: JobSchedule;
  scheduleLabel: string;
  enabled: boolean;
  status: JobStatus;
  nextRunAt?: string;
  lastRunAt?: string;
  lastRunStatus?: RunStatus;
  lastRunId?: LocalId;
}

export interface JobFormInput {
  sourceId: LocalId;
  name: string;
  enabled: boolean;
  schedule: JobSchedule;
}

export interface RunCounts {
  processed: number;
  synced: number;
  skipped: number;
  failed: number;
  deleted: number;
}

export interface SyncErrorDto {
  id: LocalId;
  runId?: LocalId;
  sourceId?: LocalId;
  sourcePath?: string;
  code: string;
  message: string;
  details?: Record<string, unknown>;
  createdAt?: string;
}

export interface SyncRunDto {
  id: LocalId;
  jobId?: LocalId;
  sourceId?: LocalId;
  sourceName: string;
  jobName?: string;
  status: RunStatus;
  startedAt: string;
  finishedAt?: string;
  durationMs?: number;
  counts: RunCounts;
  errors: SyncErrorDto[];
}

export type ItemType = "file" | "directory" | "virtual_document";

export interface SyncItemDto {
  id: LocalId;
  sourceId: LocalId;
  sourcePath: string;
  itemType: ItemType;
  status: ItemSyncStatus;
  size?: number;
  etag?: string;
  modifiedAt?: string;
  contentHash?: string;
  metadataJson?: unknown;
}

export interface ItemFilters {
  runId?: LocalId;
  sourceId?: LocalId;
  status?: ItemSyncStatus;
}

export interface ErrorFilters {
  runId?: LocalId;
  sourceId?: LocalId;
}

export interface SettingsDto {
  vaultPath: string;
  databasePath: string;
  listenAddress: string;
  jobConcurrency: number;
  fileConcurrency: number;
  logLevel: LogLevel;
  readOnly: {
    vaultPath: boolean;
    databasePath: boolean;
    listenAddress: boolean;
  };
}

export type SettingsUpdate = Pick<SettingsDto, "jobConcurrency" | "fileConcurrency" | "logLevel">;

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
