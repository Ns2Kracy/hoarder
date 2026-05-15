import { derived, get, writable } from "svelte/store";
import { api } from "./api";
import type {
  ApiData,
  ConsoleSummary,
  ErrorFilters,
  ItemFilters,
  JobFormInput,
  Loadable,
  SettingsDto,
  SettingsUpdate,
  SourceDto,
  SourceFormInput,
  SyncErrorDto,
  SyncItemDto,
  SyncJobDto,
  SyncRunDto,
} from "./types";

const emptyList = <T>(): Loadable<T[]> => ({
  status: "idle",
  data: [],
  origin: "mock",
});

const emptyValue = <T>(data: T): Loadable<T> => ({
  status: "idle",
  data,
  origin: "mock",
});

const defaultSettings: SettingsDto = {
  vaultPath: "",
  databasePath: "",
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

export const sources = writable<Loadable<SourceDto[]>>(emptyList());
export const jobs = writable<Loadable<SyncJobDto[]>>(emptyList());
export const runs = writable<Loadable<SyncRunDto[]>>(emptyList());
export const selectedRunDetail = writable<Loadable<SyncRunDto | undefined>>(emptyValue(undefined));
export const runItems = writable<Loadable<SyncItemDto[]>>(emptyList());
export const runErrors = writable<Loadable<SyncErrorDto[]>>(emptyList());
export const settings = writable<Loadable<SettingsDto>>(emptyValue(defaultSettings));

export const summary = derived([sources, jobs, runs], ([$sources, $jobs, $runs]) =>
  summarizeConsole($sources.data, $jobs.data, $runs.data),
);

export const consoleOrigin = derived([sources, jobs, runs, settings], (loadables) =>
  loadables.some((loadable) => loadable.origin === "api") ? "api" : "mock",
);

export const isRefreshing = derived([sources, jobs, runs, settings], (loadables) =>
  loadables.some((loadable) => loadable.status === "loading"),
);

function statusFor<T>(result: ApiData<T[]>) {
  if (result.data.length === 0) {
    return "empty" as const;
  }

  return "ready" as const;
}

function applyResult<T>(
  result: ApiData<T>,
  fallbackStatus: "ready" | "empty" = "ready",
): Loadable<T> {
  return {
    status: fallbackStatus,
    data: result.data,
    origin: result.origin,
    error: result.error,
    updatedAt: new Date().toISOString(),
  };
}

export async function loadConsoleData() {
  sources.update((current) => ({ ...current, status: "loading" }));
  jobs.update((current) => ({ ...current, status: "loading" }));
  runs.update((current) => ({ ...current, status: "loading" }));
  settings.update((current) => ({ ...current, status: "loading" }));

  const sourceResultPromise = api.getSources();
  const settingsResultPromise = api.getSettings();

  const sourceResult = await sourceResultPromise;
  const jobResult = await api.getJobs(sourceResult.data);
  const runResult = await api.getRuns(jobResult.data);
  const settingsResult = await settingsResultPromise;

  sources.set(applyResult(sourceResult, statusFor(sourceResult)));
  jobs.set(applyResult(jobResult, statusFor(jobResult)));
  runs.set(applyResult(runResult, statusFor(runResult)));
  settings.set(applyResult(settingsResult));
}

export async function addSource(input: SourceFormInput) {
  const result = await api.createSource(input);
  sources.update((current) => ({
    ...current,
    status: "ready",
    origin: result.origin,
    error: result.error,
    data: [result.data, ...current.data],
    updatedAt: new Date().toISOString(),
  }));
}

export async function testSourceConnection(sourceId: string) {
  const result = await api.testSource(sourceId);
  sources.update((current) => ({
    ...current,
    origin: result.origin,
    error: result.error,
    data: current.data.map((source) =>
      source.id === sourceId
        ? {
            ...source,
            health: result.data.ok ? "healthy" : "failed",
            lastCheckedAt: result.data.checkedAt,
          }
        : source,
    ),
    updatedAt: new Date().toISOString(),
  }));
}

export async function createJob(input: JobFormInput) {
  const result = await api.createJob(input, get(sources).data);
  jobs.update((current) => ({
    ...current,
    status: "ready",
    origin: result.origin,
    error: result.error,
    data: [result.data, ...current.data.filter((job) => job.id !== result.data.id)],
    updatedAt: new Date().toISOString(),
  }));
}

export async function triggerJobRun(jobId: string) {
  const runResult = await api.runJob(jobId, get(jobs).data);
  const jobResult = await api.getJobs(get(sources).data);
  const runListResult = await api.getRuns(jobResult.data);
  const refreshedRuns = upsertRun(runListResult.data, runResult.data);

  runs.set(
    applyResult(
      {
        ...runListResult,
        data: refreshedRuns,
        error: runListResult.error ?? runResult.error,
      },
      statusFor({ ...runListResult, data: refreshedRuns }),
    ),
  );
  jobs.set(
    applyResult(
      {
        ...jobResult,
        error: jobResult.error ?? runResult.error,
      },
      statusFor(jobResult),
    ),
  );
}

export async function loadRunDetail(runId: string, filters: Omit<ItemFilters, "runId"> = {}) {
  selectedRunDetail.update((current) => ({ ...current, status: "loading" }));
  runItems.update((current) => ({ ...current, status: "loading" }));
  runErrors.update((current) => ({ ...current, status: "loading" }));

  const itemFilters: ItemFilters = { ...filters, runId };
  const errorFilters: ErrorFilters = {
    runId,
    sourceId: filters.sourceId,
  };
  const [detailResult, itemResult, errorResult] = await Promise.all([
    api.getRunDetail(runId, get(jobs).data, get(runs).data),
    api.getItems(itemFilters),
    api.getErrors(errorFilters),
  ]);
  const detail = {
    ...detailResult.data,
    errors: errorResult.data,
  };

  selectedRunDetail.set(
    applyResult({
      ...detailResult,
      data: detail,
      error: detailResult.error ?? errorResult.error,
    }),
  );
  runItems.set(applyResult(itemResult, statusFor(itemResult)));
  runErrors.set(applyResult(errorResult, statusFor(errorResult)));
  runs.update((current) => ({
    ...current,
    data: upsertRun(current.data, detail),
    updatedAt: new Date().toISOString(),
  }));
}

export async function saveSettings(nextSettings: SettingsUpdate) {
  const result = await api.updateSettings(nextSettings);
  settings.set(applyResult(result));
}

export function summarizeConsole(
  sourceList: SourceDto[],
  jobList: SyncJobDto[],
  runList: SyncRunDto[],
): ConsoleSummary {
  const sortedRuns = [...runList].sort(
    (left, right) => Date.parse(right.startedAt) - Date.parse(left.startedAt),
  );
  const failedItemCount = runList.reduce((total, run) => total + run.counts.failed, 0);
  const syncedItems = runList.reduce((total, run) => total + run.counts.synced, 0);

  return {
    sourceCount: sourceList.length,
    enabledSourceCount: sourceList.filter((source) => source.enabled).length,
    activeJobCount: jobList.filter((job) => job.enabled).length,
    runningJobCount: jobList.filter((job) => job.status === "running").length,
    failedItemCount,
    vaultSizeLabel: estimateVaultSize(syncedItems),
    lastRun: sortedRuns[0],
  };
}

function estimateVaultSize(syncedItems: number) {
  const estimatedBytes = syncedItems * 728_000;
  if (estimatedBytes < 1024 * 1024 * 1024) {
    return `${(estimatedBytes / 1024 / 1024).toFixed(1)} MB`;
  }

  return `${(estimatedBytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

export function isEmptyLoadable<T>(loadable: Loadable<T[]>) {
  return loadable.status === "empty" || (loadable.status === "ready" && loadable.data.length === 0);
}

function upsertRun(runList: SyncRunDto[], run: SyncRunDto) {
  return [run, ...runList.filter((candidate) => candidate.id !== run.id)];
}
