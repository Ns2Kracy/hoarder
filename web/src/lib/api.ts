import type {
  ApiData,
  ApiErrorBody,
  FrontendApiError,
  SettingsDto,
  SettingsUpdate,
  SourceDto,
  SourceFormInput,
  SyncJobDto,
  SyncRunDto
} from "./types";

const API_BASE = "/api";
const REDACTED = "redacted";

const now = new Date("2026-05-12T09:24:00+08:00");

const isoMinutesAgo = (minutes: number) => new Date(now.getTime() - minutes * 60_000).toISOString();
const isoMinutesAhead = (minutes: number) => new Date(now.getTime() + minutes * 60_000).toISOString();

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
    lastRunAt: isoMinutesAgo(32)
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
      secret_access_key: REDACTED
    },
    health: "warning",
    itemCount: 6432,
    lastCheckedAt: isoMinutesAgo(84),
    lastRunAt: isoMinutesAgo(102),
    lastError: "Two objects were skipped because their source paths normalize into reserved vault paths."
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
      token: REDACTED
    },
    health: "disabled",
    itemCount: 0,
    lastCheckedAt: isoMinutesAgo(1500)
  }
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
    lastRunStatus: "completed"
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
    lastRunStatus: "running"
  },
  {
    id: "job-webdav-paused",
    sourceId: "src-webdav-research",
    sourceName: "Research WebDAV",
    schedule: "Manual",
    enabled: false,
    status: "paused",
    lastRunAt: isoMinutesAgo(1500),
    lastRunStatus: "failed"
  }
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
      deleted: 14
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
          policy: "mark_failed_continue_run"
        },
        createdAt: isoMinutesAgo(8)
      }
    ]
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
      deleted: 0
    },
    errors: []
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
      deleted: 0
    },
    errors: [
      {
        id: "err-webdav-auth",
        runId: "run-20260511-025",
        code: "CONNECTOR_AUTH_FAILED",
        message: "WebDAV token was rejected by the remote server.",
        details: {
          endpoint: "https://dav.example.test/research",
          status: 401
        },
        createdAt: isoMinutesAgo(1501)
      }
    ]
  }
];

const mockSettings: SettingsDto = {
  vaultPath: "/Users/alex/HoarderVault",
  databasePath: "/Users/alex/Library/Application Support/hoarder/hoarder.sqlite",
  listenAddress: "127.0.0.1:4761",
  jobConcurrency: 1,
  fileConcurrency: 4,
  logLevel: "info"
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
      status
    };
  }

  if (error instanceof Error) {
    return {
      code: "NETWORK_ERROR",
      message: error.message,
      status
    };
  }

  return {
    code: "UNKNOWN_ERROR",
    message: "The local API returned an unexpected error.",
    status
  };
}

function isFrontendApiError(value: unknown): value is FrontendApiError {
  return Boolean(
    value &&
      typeof value === "object" &&
      typeof (value as { code?: unknown }).code === "string" &&
      typeof (value as { message?: unknown }).message === "string"
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
      typeof (maybeError as { message?: unknown }).message === "string"
  );
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE}${path}`, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
      ...init?.headers
    },
    ...init
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
      status: response.status
    } satisfies FrontendApiError;
  }

  return body as T;
}

async function withMockFallback<T>(loader: () => Promise<T>, fallback: () => T): Promise<ApiData<T>> {
  try {
    return { data: await loader(), origin: "api" };
  } catch (error) {
    return { data: fallback(), origin: "mock", error: normalizeApiError(error) };
  }
}

export const api = {
  getSources: () => withMockFallback(() => request<SourceDto[]>("/sources"), () => [...mockSources]),

  createSource: async (input: SourceFormInput): Promise<ApiData<SourceDto>> =>
    withMockFallback(
      () =>
        request<SourceDto>("/sources", {
          method: "POST",
          body: JSON.stringify(toSourceRequest(input))
        }),
      () => {
        const created = {
          id: `src-${input.name.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/(^-|-$)/g, "") || "new"}`,
          name: input.name,
          connectorKind: "opendal" as const,
          serviceKind: input.serviceKind,
          enabled: input.enabled,
          config: redactConfig(input),
          health: "untested" as const,
          itemCount: 0
        };
        mockSources.unshift(created);
        return created;
      }
    ),

  testSource: async (sourceId: string): Promise<ApiData<{ ok: boolean; checkedAt: string }>> =>
    withMockFallback(
      () => request<{ ok: boolean; checkedAt: string }>(`/sources/${sourceId}/test`, { method: "POST" }),
      () => {
        const checkedAt = new Date().toISOString();
        const source = mockSources.find((candidate) => candidate.id === sourceId);
        if (source) {
          source.health = "healthy";
          source.lastCheckedAt = checkedAt;
        }
        return { ok: true, checkedAt };
      }
    ),

  getJobs: () => withMockFallback(() => request<SyncJobDto[]>("/jobs"), () => [...mockJobs]),

  runJob: async (jobId: string): Promise<ApiData<SyncRunDto>> =>
    withMockFallback(
      () => request<SyncRunDto>(`/jobs/${jobId}/run`, { method: "POST" }),
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
            deleted: 0
          },
          errors: []
        };
        mockRuns.unshift(run);
        if (job) {
          job.status = "running";
          job.lastRunAt = run.startedAt;
          job.lastRunStatus = run.status;
        }
        return run;
      }
    ),

  getRuns: () => withMockFallback(() => request<SyncRunDto[]>("/runs"), () => [...mockRuns]),

  getSettings: () => withMockFallback(() => request<SettingsDto>("/settings"), () => ({ ...mockSettings })),

  updateSettings: async (settings: SettingsUpdate): Promise<ApiData<SettingsDto>> =>
    withMockFallback(
      () =>
        request<SettingsDto>("/settings", {
          method: "PATCH",
          body: JSON.stringify(settings)
        }),
      () => {
        Object.assign(mockSettings, settings);
        return { ...mockSettings };
      }
    )
};

function toSourceRequest(input: SourceFormInput) {
  return {
    name: input.name,
    connector_kind: "opendal",
    service_kind: input.serviceKind,
    enabled: input.enabled,
    config: input.config
  };
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
    token: config.token ? REDACTED : undefined
  };
}
