import type {
  ApiData,
  ApiErrorBody,
  FrontendApiError,
  SettingsDto,
  SettingsUpdate,
  SourceDto,
  SourceFormInput,
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
    schedule: "Every 60 minutes",
    enabled: true,
    status: "scheduled",
    nextRunAt: isoMinutesAhead(26),
    lastRunAt: isoMinutesAgo(32),
    lastRunStatus: "completed",
  },
  {
    id: "job-s3-nightly",
    sourceId: "src-team-s3",
    sourceName: "Team S3 archive",
    schedule: "Daily at 02:00",
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
    schedule: "Manual",
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

const mockSettings: SettingsDto = {
  vaultPath: "/Users/alex/HoarderVault",
  databasePath: "/Users/alex/Library/Application Support/hoarder/hoarder.sqlite",
  listenAddress: "127.0.0.1:4761",
  jobConcurrency: 1,
  fileConcurrency: 4,
  logLevel: "info",
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
}

interface BackendJobDto {
  id: string;
  sourceId: string;
  name: string;
  enabled: boolean;
  schedule?: string | null;
}

interface BackendRunDto {
  id: string;
  jobId: string;
  status: "pending" | "synced" | "failed" | "skipped" | "deleted_on_source";
  startedAt?: string | null;
  finishedAt?: string | null;
  processedCount: number;
  syncedCount: number;
  skippedCount: number;
  failedCount: number;
}

interface BackendJobRunResponse {
  runId: string;
  status: "pending" | "synced" | "failed" | "skipped" | "deleted_on_source";
}

interface BackendSettingsDto {
  databasePath: string;
  vaultPath: string;
  listenAddr: string;
  jobConcurrency: number;
  fileConcurrency: number;
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

  getJobs: () =>
    withMockFallback(
      async () => {
        const sources = await api.getSources();
        const sourceNames = new Map(sources.data.map((source) => [source.id, source.name]));
        const response = await request<BackendListResponse<BackendJobDto>>("/jobs");
        return response.data.map((job) => toJobDto(job, sourceNames));
      },
      () => [...mockJobs],
    ),

  runJob: async (jobId: string): Promise<ApiData<SyncRunDto>> =>
    withMockFallback(
      async () => {
        const response = await request<BackendJobRunResponse>(`/jobs/${jobId}/run`, {
          method: "POST",
        });
        return runResponseToRunDto(jobId, response);
      },
      () => {
        const job = mockJobs.find((candidate) => candidate.id === jobId);
        const run: SyncRunDto = {
          id: `run-${Date.now()}`,
          jobId,
          sourceId: job?.sourceId ?? "unknown-source",
          sourceName: job?.sourceName ?? "Unknown source",
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

  getRuns: () =>
    withMockFallback(
      async () => {
        const jobs = await api.getJobs();
        const jobsById = new Map(jobs.data.map((job) => [job.id, job]));
        const response = await request<BackendListResponse<BackendRunDto>>("/runs");
        return response.data.map((run) => toRunDto(run, jobsById));
      },
      () => [...mockRuns],
    ),

  getSettings: () =>
    withMockFallback(
      async () => toSettingsDto(await request<BackendSettingsDto>("/settings")),
      () => ({ ...mockSettings }),
    ),

  updateSettings: async (settings: SettingsUpdate): Promise<ApiData<SettingsDto>> =>
    withMockFallback(
      () =>
        request<SettingsDto>("/settings", {
          method: "PATCH",
          body: JSON.stringify(settings),
        }),
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
    health: source.enabled ? "untested" : "disabled",
    itemCount: 0,
  };
}

function toJobDto(job: BackendJobDto, sourceNames: Map<string, string>): SyncJobDto {
  return {
    id: job.id,
    sourceId: job.sourceId,
    sourceName: sourceNames.get(job.sourceId) ?? job.sourceId,
    schedule: job.schedule ?? "Manual",
    enabled: job.enabled,
    status: job.enabled ? "scheduled" : "paused",
  };
}

function toRunDto(run: BackendRunDto, jobsById: Map<string, SyncJobDto>): SyncRunDto {
  const job = jobsById.get(run.jobId);

  return {
    id: run.id,
    jobId: run.jobId,
    sourceId: job?.sourceId ?? "unknown-source",
    sourceName: job?.sourceName ?? "Unknown source",
    status: backendRunStatus(run.status),
    startedAt: run.startedAt ?? new Date().toISOString(),
    finishedAt: run.finishedAt ?? undefined,
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

function runResponseToRunDto(jobId: string, response: BackendJobRunResponse): SyncRunDto {
  const job = mockJobs.find((candidate) => candidate.id === jobId);

  return {
    id: response.runId,
    jobId,
    sourceId: job?.sourceId ?? "unknown-source",
    sourceName: job?.sourceName ?? "Unknown source",
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

function toSettingsDto(settings: BackendSettingsDto): SettingsDto {
  return {
    vaultPath: settings.vaultPath,
    databasePath: settings.databasePath,
    listenAddress: settings.listenAddr,
    jobConcurrency: settings.jobConcurrency,
    fileConcurrency: settings.fileConcurrency,
    logLevel: "info",
  };
}

function backendRunStatus(status: BackendRunDto["status"]): SyncRunDto["status"] {
  if (status === "pending") {
    return "running";
  }

  if (status === "synced" || status === "skipped" || status === "deleted_on_source") {
    return "completed";
  }

  return "failed";
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
