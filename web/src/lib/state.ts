import { derived, get, writable } from "svelte/store";
import { api, normalizeApiError } from "./api";
import type {
  ApiData,
  ConsoleSummary,
  ErrorFilters,
  FileBrowseDto,
  FileBrowseFilters,
  ItemFilters,
  JobFormInput,
  LocalId,
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
export const fileBrowser = writable<Loadable<FileBrowseDto | undefined>>(emptyValue(undefined));
export const settings = writable<Loadable<SettingsDto>>(emptyValue(defaultSettings));
export const jobActions = writable({
  runningJobIds: [] as LocalId[],
  stoppingJobIds: [] as LocalId[],
});

let runDetailRequestSequence = 0;

const STOP_REFRESH_ATTEMPTS = 5;
const STOP_REFRESH_DELAY_MS = 900;

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

  let sourceResult: Awaited<ReturnType<typeof api.getSources>> | undefined;
  try {
    sourceResult = await sourceResultPromise;
    sources.set(applyResult(sourceResult, statusFor(sourceResult)));
  } catch (error) {
    sources.update((current) => loadableWithError(current, error));
  }

  try {
    const jobResult = await api.getJobs(sourceResult?.data ?? get(sources).data);
    jobs.set(applyResult(jobResult, statusFor(jobResult)));
    reconcileJobActions(jobResult.data);
  } catch (error) {
    jobs.update((current) => loadableWithError(current, error));
  }

  try {
    const runResult = await api.getRuns();
    runs.set(applyResult(runResult, statusFor(runResult)));
  } catch (error) {
    runs.update((current) => loadableWithError(current, error));
  }

  try {
    const settingsResult = await settingsResultPromise;
    settings.set(applyResult(settingsResult));
  } catch (error) {
    settings.update((current) => loadableWithError(current, error));
  }
}

export async function addSource(input: SourceFormInput) {
  try {
    const result = await api.createSource(input);
    sources.update((current) => ({
      ...current,
      status: "ready",
      origin: result.origin,
      error: result.error,
      data: [result.data, ...current.data],
      updatedAt: new Date().toISOString(),
    }));
  } catch (error) {
    sources.update((current) => loadableWithError(current, error));
  }
}

export async function updateSource(sourceId: LocalId, input: SourceFormInput) {
  try {
    const result = await api.updateSource(sourceId, input);
    const updatedAt = new Date().toISOString();
    sources.update((current) => ({
      ...current,
      status: "ready",
      origin: result.origin,
      error: result.error,
      data: current.data.some((source) => source.id === sourceId)
        ? current.data.map((source) => (source.id === sourceId ? result.data : source))
        : [result.data, ...current.data],
      updatedAt,
    }));
    jobs.update((current) => ({
      ...current,
      data: current.data.map((job) =>
        job.sourceId === sourceId ? { ...job, sourceName: result.data.name } : job,
      ),
      updatedAt,
    }));
  } catch (error) {
    sources.update((current) => loadableWithError(current, error));
  }
}

export async function deleteSource(sourceId: LocalId) {
  try {
    const result = await api.deleteSource(sourceId);
    const updatedAt = new Date().toISOString();
    sources.update((current) => {
      const nextData = current.data.filter((source) => source.id !== sourceId);
      return {
        ...current,
        status: nextData.length > 0 ? "ready" : "empty",
        origin: result.origin,
        error: result.error,
        data: nextData,
        updatedAt,
      };
    });
    jobs.update((current) => {
      const nextData = current.data.filter((job) => job.sourceId !== sourceId);
      return {
        ...current,
        status: nextData.length > 0 ? current.status : "empty",
        origin: result.origin,
        error: result.error,
        data: nextData,
        updatedAt,
      };
    });
    fileBrowser.update((current) =>
      current.data?.sourceId === sourceId
        ? { ...current, status: "idle", data: undefined, updatedAt }
        : current,
    );
  } catch (error) {
    sources.update((current) => loadableWithError(current, error));
  }
}

export async function testSourceConnection(sourceId: LocalId) {
  try {
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
  } catch (error) {
    sources.update((current) => loadableWithError(current, error));
  }
}

export async function createJob(input: JobFormInput) {
  try {
    const result = await api.createJob(input, get(sources).data);
    jobs.update((current) => ({
      ...current,
      status: "ready",
      origin: result.origin,
      error: result.error,
      data: [result.data, ...current.data.filter((job) => job.id !== result.data.id)],
      updatedAt: new Date().toISOString(),
    }));
  } catch (error) {
    jobs.update((current) => loadableWithError(current, error));
  }
}

export async function updateJob(jobId: LocalId, input: JobFormInput) {
  try {
    const result = await api.updateJob(jobId, input, get(sources).data);
    jobs.update((current) => ({
      ...current,
      status: "ready",
      origin: result.origin,
      error: result.error,
      data: current.data.some((job) => job.id === jobId)
        ? current.data.map((job) => (job.id === jobId ? result.data : job))
        : [result.data, ...current.data],
      updatedAt: new Date().toISOString(),
    }));
  } catch (error) {
    jobs.update((current) => loadableWithError(current, error));
  }
}

export async function triggerJobRun(jobId: LocalId) {
  if (get(jobActions).runningJobIds.includes(jobId)) {
    return;
  }

  addJobAction("runningJobIds", jobId);
  markJobStatus(jobId, "running");

  try {
    const runResult = await api.runJob(jobId, get(jobs).data);
    const jobResult = await api.getJobs(get(sources).data);
    const runListResult = await api.getRuns();
    const refreshedRuns = runListResult.data.some((run) => run.id === runResult.data.id)
      ? runListResult.data
      : upsertRun(runListResult.data, runResult.data);

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
    reconcileJobActions(jobResult.data);
  } catch (error) {
    jobs.update((current) => loadableWithError(current, error));
    runs.update((current) => loadableWithError(current, error));
  } finally {
    removeJobAction("runningJobIds", jobId);
  }
}

export async function stopJob(jobId: LocalId) {
  if (get(jobActions).stoppingJobIds.includes(jobId)) {
    return;
  }

  addJobAction("stoppingJobIds", jobId);

  try {
    const stopResult = await api.stopJob(jobId);
    await refreshJobsAndRuns(stopResult);

    if (get(jobs).data.find((job) => job.id === jobId)?.status !== "running") {
      removeJobAction("stoppingJobIds", jobId);
      return;
    }

    await waitForJobToStop(jobId, stopResult);
  } catch (error) {
    removeJobAction("stoppingJobIds", jobId);
    jobs.update((current) => loadableWithError(current, error));
    runs.update((current) => loadableWithError(current, error));
  }
}

export async function loadRunDetail(runId: LocalId, filters: Omit<ItemFilters, "runId"> = {}) {
  const requestSequence = ++runDetailRequestSequence;

  selectedRunDetail.update((current) => ({ ...current, status: "loading", data: undefined }));
  runItems.update((current) => ({ ...current, status: "loading", data: [] }));
  runErrors.update((current) => ({ ...current, status: "loading", data: [] }));

  const itemFilters: ItemFilters = { ...filters, runId };
  const errorFilters: ErrorFilters = {
    runId,
    sourceId: filters.sourceId,
  };
  let detailResult: Awaited<ReturnType<typeof api.getRunDetail>>;
  let itemResult: Awaited<ReturnType<typeof api.getItems>>;
  let errorResult: Awaited<ReturnType<typeof api.getErrors>>;

  try {
    [detailResult, itemResult, errorResult] = await Promise.all([
      api.getRunDetail(runId, get(jobs).data, get(runs).data),
      api.getItems(itemFilters),
      api.getErrors(errorFilters),
    ]);
  } catch (error) {
    if (requestSequence === runDetailRequestSequence) {
      selectedRunDetail.update((current) => loadableWithError(current, error));
      runItems.update((current) => loadableWithError(current, error));
      runErrors.update((current) => loadableWithError(current, error));
    }

    return;
  }

  if (requestSequence !== runDetailRequestSequence) {
    return;
  }

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

export async function loadFiles(filters: FileBrowseFilters) {
  fileBrowser.update((current) => ({ ...current, status: "loading" }));

  try {
    const result = await api.getFiles(filters);
    fileBrowser.set(applyResult(result));
  } catch (error) {
    fileBrowser.update((current) => loadableWithError(current, error));
  }
}

export async function saveSettings(nextSettings: SettingsUpdate) {
  try {
    const result = await api.updateSettings(nextSettings);
    settings.set(applyResult(result));
  } catch (error) {
    settings.update((current) => loadableWithError(current, error));
  }
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

function addJobAction(key: "runningJobIds" | "stoppingJobIds", jobId: LocalId) {
  jobActions.update((current) =>
    current[key].includes(jobId)
      ? current
      : {
          ...current,
          [key]: [...current[key], jobId],
        },
  );
}

function removeJobAction(key: "runningJobIds" | "stoppingJobIds", jobId: LocalId) {
  jobActions.update((current) => ({
    ...current,
    [key]: current[key].filter((candidate) => candidate !== jobId),
  }));
}

function markJobStatus(jobId: LocalId, status: SyncJobDto["status"]) {
  jobs.update((current) => ({
    ...current,
    data: current.data.map((job) => (job.id === jobId ? { ...job, status } : job)),
    updatedAt: new Date().toISOString(),
  }));
}

function reconcileJobActions(jobList: SyncJobDto[]) {
  const runningJobIds = new Set(
    jobList.filter((job) => job.status === "running").map((job) => job.id),
  );
  jobActions.update((current) => ({
    runningJobIds: current.runningJobIds.filter((jobId) => runningJobIds.has(jobId)),
    stoppingJobIds: current.stoppingJobIds.filter((jobId) => runningJobIds.has(jobId)),
  }));
}

async function refreshJobsAndRuns(actionResult?: ApiData<void>) {
  const jobResult = await api.getJobs(get(sources).data);
  const runResult = await api.getRuns();

  jobs.set(
    applyResult(
      {
        ...jobResult,
        error: jobResult.error ?? actionResult?.error,
      },
      statusFor(jobResult),
    ),
  );
  reconcileJobActions(jobResult.data);
  runs.set(
    applyResult(
      {
        ...runResult,
        error: runResult.error ?? actionResult?.error,
      },
      statusFor(runResult),
    ),
  );
}

async function waitForJobToStop(jobId: LocalId, actionResult?: ApiData<void>) {
  for (let attempt = 0; attempt < STOP_REFRESH_ATTEMPTS; attempt += 1) {
    await delay(STOP_REFRESH_DELAY_MS);
    await refreshJobsAndRuns(actionResult);

    if (get(jobs).data.find((job) => job.id === jobId)?.status !== "running") {
      removeJobAction("stoppingJobIds", jobId);
      return;
    }
  }

  removeJobAction("stoppingJobIds", jobId);
}

function delay(milliseconds: number) {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

function loadableWithError<T>(loadable: Loadable<T>, error: unknown): Loadable<T> {
  return {
    ...loadable,
    status: Array.isArray(loadable.data) && loadable.data.length === 0 ? "empty" : "ready",
    error: normalizeApiError(error),
    updatedAt: new Date().toISOString(),
  };
}
