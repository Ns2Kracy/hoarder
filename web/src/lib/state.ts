import { derived, writable } from "svelte/store";
import { api } from "./api";
import type {
  ApiData,
  ConsoleSummary,
  Loadable,
  SettingsDto,
  SourceDto,
  SourceFormInput,
  SyncJobDto,
  SyncRunDto
} from "./types";

const emptyList = <T>(): Loadable<T[]> => ({
  status: "idle",
  data: [],
  origin: "mock"
});

const emptyValue = <T>(data: T): Loadable<T> => ({
  status: "idle",
  data,
  origin: "mock"
});

const defaultSettings: SettingsDto = {
  vaultPath: "",
  databasePath: "",
  listenAddress: "127.0.0.1:4761",
  jobConcurrency: 1,
  fileConcurrency: 4,
  logLevel: "info"
};

export const sources = writable<Loadable<SourceDto[]>>(emptyList());
export const jobs = writable<Loadable<SyncJobDto[]>>(emptyList());
export const runs = writable<Loadable<SyncRunDto[]>>(emptyList());
export const settings = writable<Loadable<SettingsDto>>(emptyValue(defaultSettings));

export const summary = derived([sources, jobs, runs], ([$sources, $jobs, $runs]) =>
  summarizeConsole($sources.data, $jobs.data, $runs.data)
);

export const consoleOrigin = derived([sources, jobs, runs, settings], (loadables) =>
  loadables.some((loadable) => loadable.origin === "api") ? "api" : "mock"
);

export const isRefreshing = derived([sources, jobs, runs, settings], (loadables) =>
  loadables.some((loadable) => loadable.status === "loading")
);

function statusFor<T>(result: ApiData<T[]>) {
  if (result.data.length === 0) {
    return "empty" as const;
  }

  return "ready" as const;
}

function applyResult<T>(result: ApiData<T>, fallbackStatus: "ready" | "empty" = "ready"): Loadable<T> {
  return {
    status: fallbackStatus,
    data: result.data,
    origin: result.origin,
    error: result.error,
    updatedAt: new Date().toISOString()
  };
}

export async function loadConsoleData() {
  sources.update((current) => ({ ...current, status: "loading" }));
  jobs.update((current) => ({ ...current, status: "loading" }));
  runs.update((current) => ({ ...current, status: "loading" }));
  settings.update((current) => ({ ...current, status: "loading" }));

  const [sourceResult, jobResult, runResult, settingsResult] = await Promise.all([
    api.getSources(),
    api.getJobs(),
    api.getRuns(),
    api.getSettings()
  ]);

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
    updatedAt: new Date().toISOString()
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
            lastCheckedAt: result.data.checkedAt
          }
        : source
    ),
    updatedAt: new Date().toISOString()
  }));
}

export async function triggerJobRun(jobId: string) {
  const result = await api.runJob(jobId);
  runs.update((current) => ({
    ...current,
    status: "ready",
    origin: result.origin,
    error: result.error,
    data: [result.data, ...current.data],
    updatedAt: new Date().toISOString()
  }));
  jobs.update((current) => ({
    ...current,
    origin: result.origin,
    error: result.error,
    data: current.data.map((job) =>
      job.id === jobId
        ? {
            ...job,
            status: "running",
            lastRunAt: result.data.startedAt,
            lastRunStatus: result.data.status
          }
        : job
    ),
    updatedAt: new Date().toISOString()
  }));
}

export async function saveSettings(nextSettings: SettingsDto) {
  const result = await api.updateSettings(nextSettings);
  settings.set(applyResult(result));
}

export function summarizeConsole(sourceList: SourceDto[], jobList: SyncJobDto[], runList: SyncRunDto[]): ConsoleSummary {
  const sortedRuns = [...runList].sort((left, right) => Date.parse(right.startedAt) - Date.parse(left.startedAt));
  const failedItemCount = runList.reduce((total, run) => total + run.counts.failed, 0);
  const syncedItems = runList.reduce((total, run) => total + run.counts.synced, 0);

  return {
    sourceCount: sourceList.length,
    enabledSourceCount: sourceList.filter((source) => source.enabled).length,
    activeJobCount: jobList.filter((job) => job.enabled).length,
    runningJobCount: jobList.filter((job) => job.status === "running").length,
    failedItemCount,
    vaultSizeLabel: estimateVaultSize(syncedItems),
    lastRun: sortedRuns[0]
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
