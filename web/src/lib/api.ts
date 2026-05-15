import type {
  ApiData,
  ApiErrorBody,
  ErrorFilters,
  FrontendApiError,
  ItemFilters,
  JobFormInput,
  SettingsDto,
  SettingsUpdate,
  SourceDto,
  SourceFormInput,
  JobSchedule,
  SyncErrorDto,
  SyncItemDto,
  SyncJobDto,
  SyncRunDto,
} from "./types";

const API_BASE = "/api";
const REDACTED = "redacted";
const DEFAULT_SERVICE_KIND = "fs";

const now = new Date("2026-05-12T09:24:00+08:00");

const isoMinutesAgo = (minutes: number) => new Date(now.getTime() - minutes * 60_000).toISOString();
const isoMinutesAhead = (minutes: number) =>
  new Date(now.getTime() + minutes * 60_000).toISOString();

const mockSources: SourceDto[] = [
  {
    id: "src-local-notes",
    name: "Local notes",
    connectorKind: "opendal",
    serviceKind: "fs",
    enabled: true,
    config: { service: "fs", root: "/Users/alex/Documents/notes" },
    health: "healthy",
    itemCount: 1284,
    lastCheckedAt: isoMinutesAgo(18),
    lastRunAt: isoMinutesAgo(32),
  },
  {
    id: "src-team-s3",
    name: "Team S3 archive",
    connectorKind: "opendal",
    serviceKind: "s3",
    enabled: true,
    config: {
      service: "s3",
      bucket: "team-archive",
      region: "us-west-2",
      access_key_id: REDACTED,
      secret_access_key: REDACTED,
    },
    health: "warning",
    itemCount: 6432,
    lastCheckedAt: isoMinutesAgo(84),
    lastRunAt: isoMinutesAgo(102),
    lastError:
      "Two objects were skipped because their source paths normalize into reserved vault paths.",
  },
  {
    id: "src-webdav-research",
    name: "Research WebDAV",
    connectorKind: "opendal",
    serviceKind: "webdav",
    enabled: false,
    config: {
      service: "webdav",
      endpoint: "https://dav.example.test/research",
      username: "alex",
      token: REDACTED,
    },
    health: "disabled",
    itemCount: 0,
    lastCheckedAt: isoMinutesAgo(1500),
  },
];

const mockJobs: SyncJobDto[] = [
  {
    id: "job-notes-hourly",
    sourceId: "src-local-notes",
    sourceName: "Local notes",
    name: "Notes hourly",
    schedule: { kind: "interval", intervalSeconds: 3600 },
    scheduleLabel: "Every 60 minutes",
    enabled: true,
    status: "idle",
    nextRunAt: isoMinutesAhead(26),
    lastRunAt: isoMinutesAgo(32),
    lastRunStatus: "completed",
  },
  {
    id: "job-s3-nightly",
    sourceId: "src-team-s3",
    sourceName: "Team S3 archive",
    name: "S3 nightly",
    schedule: { kind: "interval", intervalSeconds: 86_400 },
    scheduleLabel: "Every 24 hours",
    enabled: true,
    status: "running",
    nextRunAt: isoMinutesAhead(977),
    lastRunAt: isoMinutesAgo(11),
    lastRunStatus: "running",
  },
  {
    id: "job-webdav-paused",
    sourceId: "src-webdav-research",
    sourceName: "Research WebDAV",
    name: "Research manual",
    schedule: { kind: "manual" },
    scheduleLabel: "Manual",
    enabled: false,
    status: "paused",
    lastRunAt: isoMinutesAgo(1500),
    lastRunStatus: "failed",
  },
];

const mockRuns: SyncRunDto[] = [
  {
    id: "run-20260512-012",
    jobId: "job-s3-nightly",
    sourceId: "src-team-s3",
    sourceName: "Team S3 archive",
    jobName: "S3 nightly",
    status: "running",
    startedAt: isoMinutesAgo(11),
    counts: {
      processed: 2218,
      synced: 184,
      skipped: 2019,
      failed: 1,
      deleted: 14,
    },
    errors: [
      {
        id: "err-s3-reserved-path",
        runId: "run-20260512-012",
        sourceId: "src-team-s3",
        sourcePath: ".hoarder/tmp/leaked",
        code: "RESERVED_TARGET_PATH",
        message: "Source item cannot write under the reserved .hoarder directory.",
        details: {
          target_path: "src-team-s3/.hoarder/tmp/leaked",
          policy: "mark_failed_continue_run",
        },
        createdAt: isoMinutesAgo(8),
      },
    ],
  },
  {
    id: "run-20260512-011",
    jobId: "job-notes-hourly",
    sourceId: "src-local-notes",
    sourceName: "Local notes",
    jobName: "Notes hourly",
    status: "completed",
    startedAt: isoMinutesAgo(35),
    finishedAt: isoMinutesAgo(32),
    durationMs: 173_000,
    counts: {
      processed: 1284,
      synced: 17,
      skipped: 1267,
      failed: 0,
      deleted: 0,
    },
    errors: [],
  },
  {
    id: "run-20260511-025",
    jobId: "job-webdav-paused",
    sourceId: "src-webdav-research",
    sourceName: "Research WebDAV",
    jobName: "Research manual",
    status: "failed",
    startedAt: isoMinutesAgo(1510),
    finishedAt: isoMinutesAgo(1500),
    durationMs: 604_000,
    counts: {
      processed: 0,
      synced: 0,
      skipped: 0,
      failed: 1,
      deleted: 0,
    },
    errors: [
      {
        id: "err-webdav-auth",
        runId: "run-20260511-025",
        sourceId: "src-webdav-research",
        code: "CONNECTOR_AUTH_FAILED",
        message: "WebDAV token was rejected by the remote server.",
        details: {
          endpoint: "https://dav.example.test/research",
          status: 401,
        },
        createdAt: isoMinutesAgo(1501),
      },
    ],
  },
];

const mockItems: SyncItemDto[] = [
  {
    id: "item-notes-readme",
    sourceId: "src-local-notes",
    sourcePath: "README.md",
    itemType: "file",
    status: "synced",
    size: 8452,
    modifiedAt: isoMinutesAgo(44),
    metadataJson: {
      runId: "run-20260512-011",
    },
  },
  {
    id: "item-notes-index",
    sourceId: "src-local-notes",
    sourcePath: "index.md",
    itemType: "file",
    status: "skipped",
    size: 1298,
    modifiedAt: isoMinutesAgo(80),
    metadataJson: {
      runId: "run-20260512-011",
    },
  },
  {
    id: "item-s3-reserved",
    sourceId: "src-team-s3",
    sourcePath: ".hoarder/tmp/leaked",
    itemType: "file",
    status: "failed",
    size: 128,
    modifiedAt: isoMinutesAgo(9),
    metadataJson: {
      runId: "run-20260512-012",
    },
  },
  {
    id: "item-s3-archive",
    sourceId: "src-team-s3",
    sourcePath: "archive/2026/report.pdf",
    itemType: "file",
    status: "synced",
    size: 2_400_000,
    modifiedAt: isoMinutesAgo(10),
    metadataJson: {
      runId: "run-20260512-012",
    },
  },
  {
    id: "item-s3-deleted",
    sourceId: "src-team-s3",
    sourcePath: "archive/old.csv",
    itemType: "file",
    status: "deleted_on_source",
    metadataJson: {
      runId: "run-20260512-012",
    },
  },
];

const mockSettings: SettingsDto = {
  vaultPath: "/Users/alex/HoarderVault",
  databasePath: "/Users/alex/Library/Application Support/hoarder/hoarder.sqlite",
  listenAddress: "127.0.0.1:4761",
  jobConcurrency: 1,
  fileConcurrency: 4,
  logLevel: "info",
  readOnly: {
    vaultPath: true,
    databasePath: true,
    listenAddress: true,
  },
};

function normalizeApiError(error: unknown, status?: number): FrontendApiError {
  if (isFrontendApiError(error)) {
    return error;
  }

  if (isApiErrorBody(error)) {
    return {
      code: error.error.code,
      message: error.error.message,
      details: error.error.details,
      status,
    };
  }

  if (error instanceof Error) {
    return {
      code: "NETWORK_ERROR",
      message: error.message,
      status,
    };
  }

  return {
    code: "UNKNOWN_ERROR",
    message: "The local API returned an unexpected error.",
    status,
  };
}

function isFrontendApiError(value: unknown): value is FrontendApiError {
  return Boolean(
    value &&
    typeof value === "object" &&
    typeof (value as { code?: unknown }).code === "string" &&
    typeof (value as { message?: unknown }).message === "string",
  );
}

function isApiErrorBody(value: unknown): value is ApiErrorBody {
  if (!value || typeof value !== "object" || !("error" in value)) {
    return false;
  }

  const maybeError = (value as { error?: unknown }).error;
  return Boolean(
    maybeError &&
    typeof maybeError === "object" &&
    typeof (maybeError as { code?: unknown }).code === "string" &&
    typeof (maybeError as { message?: unknown }).message === "string",
  );
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE}${path}`, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
      ...init?.headers,
    },
    ...init,
  });

  let body: unknown = undefined;
  const contentType = response.headers.get("content-type") ?? "";
  if (contentType.includes("application/json")) {
    body = await response.json();
  }

  if (!response.ok) {
    throw normalizeApiError(body, response.status);
  }

  if (!contentType.includes("application/json")) {
    throw {
      code: "API_UNAVAILABLE",
      message: "The local Hoarder API is not serving JSON yet.",
      status: response.status,
    } satisfies FrontendApiError;
  }

  return body as T;
}

async function withMockFallback<T>(
  loader: () => Promise<T>,
  fallback: () => T,
): Promise<ApiData<T>> {
  try {
    return { data: await loader(), origin: "api" };
  } catch (error) {
    return {
      data: fallback(),
      origin: "mock",
      error: normalizeApiError(error),
    };
  }
}

interface BackendListResponse<T> {
  data: T[];
}

interface BackendSourceDto {
  id: string;
  name: string;
  connectorKind: "opendal";
  config: {
    service: string;
    options: Record<string, string>;
  };
  enabled: boolean;
  health?: SourceDto["health"];
  lastCheckedAt?: string | null;
}

interface BackendJobDto {
  id: string;
  sourceId: string;
  name?: string;
  enabled: boolean;
  schedule?: JobSchedule | string | null;
  status?: SyncJobDto["status"] | null;
  nextRunAt?: string | null;
  lastRunAt?: string | null;
  lastRunStatus?: SyncRunDto["status"] | null;
  lastRunId?: string | null;
}

interface BackendRunDto {
  id: string;
  jobId: string;
  status: SyncRunDto["status"] | "pending" | "synced" | "skipped" | "deleted_on_source";
  startedAt?: string | null;
  finishedAt?: string | null;
  processedCount: number;
  syncedCount: number;
  skippedCount: number;
  failedCount: number;
}

interface BackendRunDetailDto {
  id: string;
  jobId: string;
  sourceId: string;
  sourceName: string;
  jobName?: string;
  status: SyncRunDto["status"];
  startedAt?: string | null;
  finishedAt?: string | null;
  durationMs?: number | null;
  counts: SyncRunDto["counts"];
  errors: BackendSyncErrorDto[];
}

interface BackendJobRunResponse {
  runId: string;
  status: SyncRunDto["status"] | "pending" | "synced" | "failed" | "skipped" | "deleted_on_source";
}

interface BackendItemDto {
  id: string;
  sourceId: string;
  sourcePath: string;
  itemType: SyncItemDto["itemType"];
  status: SyncItemDto["status"];
  size?: number | null;
  etag?: string | null;
  modifiedAt?: string | null;
  contentHash?: string | null;
  metadataJson?: unknown;
}

interface BackendSyncErrorDto {
  id: string;
  runId?: string | null;
  sourceId?: string | null;
  sourcePath?: string | null;
  code: string;
  message: string;
  details?: Record<string, unknown> | null;
  createdAt?: string | null;
}

interface BackendSettingsDto {
  databasePath: string;
  vaultPath: string;
  listenAddr: string;
  jobConcurrency: number;
  fileConcurrency: number;
  logLevel?: SettingsDto["logLevel"];
  readOnly?: {
    databasePath?: boolean;
    vaultPath?: boolean;
    listenAddr?: boolean;
  };
}

export const api = {
  getSources: () =>
    withMockFallback(
      async () => {
        const response = await request<BackendListResponse<BackendSourceDto>>("/sources");
        return response.data.map(toSourceDto);
      },
      () => [...mockSources],
    ),

  createSource: async (input: SourceFormInput): Promise<ApiData<SourceDto>> =>
    withMockFallback(
      async () => {
        const response = await request<BackendSourceDto>("/sources", {
          method: "POST",
          body: JSON.stringify(toSourceRequest(input)),
        });
        return toSourceDto(response);
      },
      () => {
        const created = {
          id: `src-${
            input.name
              .toLowerCase()
              .replace(/[^a-z0-9]+/g, "-")
              .replace(/(^-|-$)/g, "") || "new"
          }`,
          name: input.name,
          connectorKind: "opendal" as const,
          serviceKind: input.serviceKind,
          enabled: input.enabled,
          config: redactConfig(input),
          health: "untested" as const,
          itemCount: 0,
        };
        mockSources.unshift(created);
        return created;
      },
    ),

  testSource: async (sourceId: string): Promise<ApiData<{ ok: boolean; checkedAt: string }>> =>
    withMockFallback(
      () =>
        request<{ ok: boolean; checkedAt: string }>(`/sources/${sourceId}/test`, {
          method: "POST",
        }),
      () => {
        const checkedAt = new Date().toISOString();
        const source = mockSources.find((candidate) => candidate.id === sourceId);
        if (source) {
          source.health = "healthy";
          source.lastCheckedAt = checkedAt;
        }
        return { ok: true, checkedAt };
      },
    ),

  getJobs: (sourceList?: SourceDto[]) =>
    withMockFallback(
      async () => {
        const resolvedSources = sourceList ?? (await api.getSources()).data;
        const sourceNames = new Map(resolvedSources.map((source) => [source.id, source.name]));
        const response = await request<BackendListResponse<BackendJobDto>>("/jobs");
        return response.data.map((job) => toJobDto(job, sourceNames));
      },
      () => [...mockJobs],
    ),

  createJob: async (input: JobFormInput, sourceList?: SourceDto[]): Promise<ApiData<SyncJobDto>> =>
    withMockFallback(
      async () => {
        const resolvedSources = sourceList ?? (await api.getSources()).data;
        const sourceNames = new Map(resolvedSources.map((source) => [source.id, source.name]));
        const response = await request<BackendJobDto>("/jobs", {
          method: "POST",
          body: JSON.stringify(toCreateJobRequest(input)),
        });
        return toJobDto(response, sourceNames);
      },
      () => {
        const source = mockSources.find((candidate) => candidate.id === input.sourceId);
        const created: SyncJobDto = {
          id: `job-${Date.now()}`,
          sourceId: input.sourceId,
          sourceName: source?.name ?? input.sourceId,
          name: input.name,
          schedule: input.schedule,
          scheduleLabel: scheduleLabel(input.schedule),
          enabled: input.enabled,
          status: input.enabled ? "idle" : "paused",
        };
        mockJobs.unshift(created);
        return created;
      },
    ),

  runJob: async (jobId: string, jobList?: SyncJobDto[]): Promise<ApiData<SyncRunDto>> =>
    withMockFallback(
      async () => {
        const response = await request<BackendJobRunResponse>(`/jobs/${jobId}/run`, {
          method: "POST",
        });
        return runResponseToRunDto(jobId, response, jobList);
      },
      () => {
        const job = mockJobs.find((candidate) => candidate.id === jobId);
        const run: SyncRunDto = {
          id: `run-${Date.now()}`,
          jobId,
          sourceId: job?.sourceId ?? "unknown-source",
          sourceName: job?.sourceName ?? "Unknown source",
          jobName: job?.name,
          status: "running",
          startedAt: new Date().toISOString(),
          counts: {
            processed: 0,
            synced: 0,
            skipped: 0,
            failed: 0,
            deleted: 0,
          },
          errors: [],
        };
        mockRuns.unshift(run);
        if (job) {
          job.status = "running";
          job.lastRunAt = run.startedAt;
          job.lastRunStatus = run.status;
        }
        return run;
      },
    ),

  getRuns: (jobList?: SyncJobDto[]) =>
    withMockFallback(
      async () => {
        const resolvedJobs = jobList ?? (await api.getJobs()).data;
        const jobsById = new Map(resolvedJobs.map((job) => [job.id, job]));
        const response = await request<BackendListResponse<BackendRunDto>>("/runs");
        return response.data.map((run) => toRunDto(run, jobsById));
      },
      () => [...mockRuns],
    ),

  getRunDetail: (runId: string, jobList?: SyncJobDto[], runList?: SyncRunDto[]) =>
    withMockFallback(
      async () => {
        const resolvedJobs = jobList ?? (await api.getJobs()).data;
        const jobsById = new Map(resolvedJobs.map((job) => [job.id, job]));
        return toRunDetailDto(await request<BackendRunDetailDto>(`/runs/${runId}`), jobsById);
      },
      () => {
        const run =
          mockRuns.find((candidate) => candidate.id === runId) ??
          runList?.find((candidate) => candidate.id === runId);
        return run ? cloneRun(run) : emptyRunDetail(runId);
      },
    ),

  getItems: (filters: ItemFilters = {}) =>
    withMockFallback(
      async () => {
        const response = await request<BackendListResponse<BackendItemDto>>(
          `/items${queryString(filters)}`,
        );
        return response.data.map(toItemDto);
      },
      () => filterMockItems(filters),
    ),

  getErrors: (filters: ErrorFilters = {}) =>
    withMockFallback(
      async () => {
        const response = await request<BackendListResponse<BackendSyncErrorDto>>(
          `/errors${queryString(filters)}`,
        );
        return response.data.map(toSyncErrorDto);
      },
      () => filterMockErrors(filters),
    ),

  getSettings: () =>
    withMockFallback(
      async () => toSettingsDto(await request<BackendSettingsDto>("/settings")),
      () => ({ ...mockSettings }),
    ),

  updateSettings: async (settings: SettingsUpdate): Promise<ApiData<SettingsDto>> =>
    withMockFallback(
      async () =>
        toSettingsDto(
          await request<BackendSettingsDto>("/settings", {
            method: "PATCH",
            body: JSON.stringify(toSettingsUpdateRequest(settings)),
          }),
        ),
      () => {
        Object.assign(mockSettings, settings);
        return { ...mockSettings };
      },
    ),
};

function toSourceRequest(input: SourceFormInput) {
  const options: Record<string, string> = {};
  for (const [key, value] of Object.entries({
    root: input.config.root,
    endpoint: input.config.endpoint,
    bucket: input.config.bucket,
    region: input.config.region,
    username: input.config.username,
    access_key_id: input.config.accessKeyId,
    secret_access_key: input.config.secretAccessKey,
    token: input.config.token,
  })) {
    if (value) {
      options[key] = value;
    }
  }

  return {
    name: input.name,
    config: {
      kind: "opendal",
      service: input.serviceKind,
      options,
    },
    enabled: input.enabled,
  };
}

function toSourceDto(source: BackendSourceDto): SourceDto {
  const serviceKind = source.config.service || DEFAULT_SERVICE_KIND;

  return {
    id: source.id,
    name: source.name,
    connectorKind: source.connectorKind,
    serviceKind: serviceKind as SourceDto["serviceKind"],
    enabled: source.enabled,
    config: {
      service: serviceKind as SourceDto["serviceKind"],
      ...source.config.options,
    },
    health: source.health ?? (source.enabled ? "untested" : "disabled"),
    itemCount: 0,
    lastCheckedAt: source.lastCheckedAt ?? undefined,
  };
}

function toCreateJobRequest(input: JobFormInput) {
  return {
    sourceId: input.sourceId,
    name: input.name,
    enabled: input.enabled,
    schedule: input.schedule,
  };
}

function toJobDto(job: BackendJobDto, sourceNames: Map<string, string>): SyncJobDto {
  const schedule = normalizeSchedule(job.schedule);
  const sourceName = sourceNames.get(job.sourceId) ?? job.sourceId;

  return {
    id: job.id,
    sourceId: job.sourceId,
    sourceName,
    name: job.name ?? sourceName,
    schedule,
    scheduleLabel: scheduleLabel(schedule),
    enabled: job.enabled,
    status: job.status ?? (job.enabled ? "idle" : "paused"),
    nextRunAt: job.nextRunAt ?? undefined,
    lastRunAt: job.lastRunAt ?? undefined,
    lastRunStatus: job.lastRunStatus ?? undefined,
    lastRunId: job.lastRunId ?? undefined,
  };
}

function toRunDto(run: BackendRunDto, jobsById: Map<string, SyncJobDto>): SyncRunDto {
  const job = jobsById.get(run.jobId);

  return {
    id: run.id,
    jobId: run.jobId,
    sourceId: job?.sourceId ?? "unknown-source",
    sourceName: job?.sourceName ?? "Unknown source",
    jobName: job?.name,
    status: backendRunStatus(run.status),
    startedAt: run.startedAt ?? new Date().toISOString(),
    finishedAt: run.finishedAt ?? undefined,
    durationMs: durationMs(run.startedAt, run.finishedAt),
    counts: {
      processed: run.processedCount,
      synced: run.syncedCount,
      skipped: run.skippedCount,
      failed: run.failedCount,
      deleted: 0,
    },
    errors: [],
  };
}

function toRunDetailDto(run: BackendRunDetailDto, jobsById: Map<string, SyncJobDto>): SyncRunDto {
  const job = jobsById.get(run.jobId);

  return {
    id: run.id,
    jobId: run.jobId,
    sourceId: run.sourceId,
    sourceName: run.sourceName,
    jobName: run.jobName ?? job?.name,
    status: backendRunStatus(run.status),
    startedAt: run.startedAt ?? new Date().toISOString(),
    finishedAt: run.finishedAt ?? undefined,
    durationMs: run.durationMs ?? durationMs(run.startedAt, run.finishedAt),
    counts: run.counts,
    errors: run.errors.map(toSyncErrorDto),
  };
}

function runResponseToRunDto(
  jobId: string,
  response: BackendJobRunResponse,
  jobList?: SyncJobDto[],
): SyncRunDto {
  const job =
    jobList?.find((candidate) => candidate.id === jobId) ??
    mockJobs.find((candidate) => candidate.id === jobId);

  return {
    id: response.runId,
    jobId,
    sourceId: job?.sourceId ?? "unknown-source",
    sourceName: job?.sourceName ?? "Unknown source",
    jobName: job?.name,
    status: backendRunStatus(response.status),
    startedAt: new Date().toISOString(),
    counts: {
      processed: 0,
      synced: 0,
      skipped: 0,
      failed: 0,
      deleted: 0,
    },
    errors: [],
  };
}

function toItemDto(item: BackendItemDto): SyncItemDto {
  return {
    id: item.id,
    sourceId: item.sourceId,
    sourcePath: item.sourcePath,
    itemType: item.itemType,
    status: item.status,
    size: item.size ?? undefined,
    etag: item.etag ?? undefined,
    modifiedAt: item.modifiedAt ?? undefined,
    contentHash: item.contentHash ?? undefined,
    metadataJson: item.metadataJson ?? undefined,
  };
}

function toSyncErrorDto(error: BackendSyncErrorDto): SyncErrorDto {
  return {
    id: error.id,
    runId: error.runId ?? undefined,
    sourceId: error.sourceId ?? undefined,
    sourcePath: error.sourcePath ?? undefined,
    code: error.code,
    message: error.message,
    details: error.details ?? undefined,
    createdAt: error.createdAt ?? undefined,
  };
}

function toSettingsDto(settings: BackendSettingsDto): SettingsDto {
  return {
    vaultPath: settings.vaultPath,
    databasePath: settings.databasePath,
    listenAddress: settings.listenAddr,
    jobConcurrency: settings.jobConcurrency,
    fileConcurrency: settings.fileConcurrency,
    logLevel: settings.logLevel ?? "info",
    readOnly: {
      databasePath: settings.readOnly?.databasePath ?? true,
      vaultPath: settings.readOnly?.vaultPath ?? true,
      listenAddress: settings.readOnly?.listenAddr ?? true,
    },
  };
}

function toSettingsUpdateRequest(settings: SettingsUpdate): SettingsUpdate {
  return {
    jobConcurrency: settings.jobConcurrency,
    fileConcurrency: settings.fileConcurrency,
    logLevel: settings.logLevel,
  };
}

function backendRunStatus(
  status: BackendRunDto["status"] | BackendJobRunResponse["status"] | BackendRunDetailDto["status"],
): SyncRunDto["status"] {
  if (
    status === "running" ||
    status === "completed" ||
    status === "completed_with_failures" ||
    status === "failed" ||
    status === "cancelled"
  ) {
    return status;
  }

  if (status === "pending") {
    return "running";
  }

  if (status === "synced" || status === "skipped" || status === "deleted_on_source") {
    return "completed";
  }

  return "failed";
}

function normalizeSchedule(schedule: BackendJobDto["schedule"]): JobSchedule {
  if (typeof schedule === "object" && schedule?.kind === "interval") {
    return {
      kind: "interval",
      intervalSeconds: Math.max(1, Math.trunc(schedule.intervalSeconds)),
    };
  }

  return { kind: "manual" };
}

function queryString(filters: object) {
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(filters)) {
    if (typeof value === "string" && value) {
      params.set(key, value);
    }
  }

  const serialized = params.toString();
  return serialized ? `?${serialized}` : "";
}

function cloneRun(run: SyncRunDto): SyncRunDto {
  return {
    ...run,
    counts: { ...run.counts },
    errors: run.errors.map((error) => ({
      ...error,
      details: error.details ? { ...error.details } : undefined,
    })),
  };
}

function emptyRunDetail(runId: string): SyncRunDto {
  return {
    id: runId,
    jobId: "unknown-job",
    sourceId: "unknown-source",
    sourceName: "Unknown source",
    status: "failed",
    startedAt: new Date().toISOString(),
    counts: {
      processed: 0,
      synced: 0,
      skipped: 0,
      failed: 0,
      deleted: 0,
    },
    errors: [],
  };
}

function filterMockItems(filters: ItemFilters): SyncItemDto[] {
  return mockItems.filter((item) => {
    if (filters.sourceId && item.sourceId !== filters.sourceId) {
      return false;
    }

    if (filters.status && item.status !== filters.status) {
      return false;
    }

    if (filters.runId && runIdForItem(item) !== filters.runId) {
      return false;
    }

    return true;
  });
}

function filterMockErrors(filters: ErrorFilters): SyncErrorDto[] {
  return mockRuns
    .flatMap((run) => run.errors)
    .filter((error) => {
      if (filters.runId && error.runId !== filters.runId) {
        return false;
      }

      if (filters.sourceId && error.sourceId !== filters.sourceId) {
        return false;
      }

      return true;
    })
    .map((error) => ({ ...error }));
}

function runIdForItem(item: SyncItemDto) {
  const metadata = item.metadataJson;
  if (metadata && typeof metadata === "object" && "runId" in metadata) {
    const runId = (metadata as { runId?: unknown }).runId;
    return typeof runId === "string" ? runId : undefined;
  }

  return undefined;
}

function durationMs(startedAt?: string | null, finishedAt?: string | null) {
  if (!startedAt || !finishedAt) {
    return undefined;
  }

  const started = Date.parse(startedAt);
  const finished = Date.parse(finishedAt);
  if (Number.isNaN(started) || Number.isNaN(finished) || finished < started) {
    return undefined;
  }

  return finished - started;
}

function redactConfig(input: SourceFormInput) {
  const config = input.config;
  return {
    service: input.serviceKind,
    root: config.root,
    endpoint: config.endpoint,
    bucket: config.bucket,
    region: config.region,
    username: config.username,
    access_key_id: config.accessKeyId ? REDACTED : undefined,
    secret_access_key: config.secretAccessKey ? REDACTED : undefined,
    token: config.token ? REDACTED : undefined,
  };
}

function scheduleLabel(schedule: JobSchedule) {
  if (schedule.kind === "manual") {
    return "Manual";
  }

  if (schedule.intervalSeconds % 3600 === 0) {
    const hours = schedule.intervalSeconds / 3600;
    return hours === 1 ? "Every hour" : `Every ${hours} hours`;
  }

  if (schedule.intervalSeconds % 60 === 0) {
    const minutes = schedule.intervalSeconds / 60;
    return minutes === 1 ? "Every minute" : `Every ${minutes} minutes`;
  }

  return `Every ${schedule.intervalSeconds} seconds`;
}
