import { afterEach, expect, test } from "bun:test";
import { get } from "svelte/store";
import { api } from "../src/lib/api";
import {
  deleteSource,
  fileBrowser,
  jobActions,
  jobs,
  loadConsoleData,
  loadFiles,
  loadRunDetail,
  runs,
  selectedRunDetail,
  sources,
  stopJob,
  testSourceConnection,
  triggerJobRun,
  updateJob,
  updateSource,
} from "../src/lib/state";

const originalFetch = globalThis.fetch;

afterEach(() => {
  globalThis.fetch = originalFetch;
  jobActions.set({ runningJobIds: [], stoppingJobIds: [] });
});

test("loadConsoleData requests each top-level resource once", async () => {
  const paths: string[] = [];

  globalThis.fetch = (async (input) => {
    const path = requestPath(input);
    paths.push(path);

    return new Response(JSON.stringify(responseFor(path)), {
      headers: {
        "Content-Type": "application/json",
      },
    });
  }) as typeof fetch;

  await loadConsoleData();

  expect(paths.sort()).toEqual(["/api/jobs", "/api/runs", "/api/settings", "/api/sources"]);
});

test("loadConsoleData keeps successful resources when one endpoint fails", async () => {
  globalThis.fetch = (async (input) => {
    const path = requestPath(input);

    if (path === "/api/runs") {
      return jsonResponse(
        {
          error: {
            code: "INTERNAL_ERROR",
            message: "internal server error",
          },
        },
        500,
      );
    }

    return jsonResponse(responseFor(path));
  }) as typeof fetch;

  await loadConsoleData();

  expect(get(sources)).toMatchObject({
    origin: "api",
    status: "ready",
    data: [{ id: "src-local", name: "Local source" }],
  });
  expect(get(jobs)).toMatchObject({
    origin: "api",
    status: "ready",
    data: [{ id: "job-local", sourceName: "Local source" }],
  });
  expect(get(runs)).toMatchObject({
    error: {
      code: "INTERNAL_ERROR",
      status: 500,
    },
  });
});

test("testSourceConnection persists healthy status after reload when api returns source health", async () => {
  let tested = false;

  globalThis.fetch = (async (input, init) => {
    const path = requestPath(input);

    if (path === "/api/sources" && (!init?.method || init.method === "GET")) {
      return new Response(
        JSON.stringify({
          data: [
            {
              id: "src-local",
              name: "Local source",
              connectorKind: "opendal",
              config: {
                service: "fs",
                options: {
                  root: "/tmp/hoarder",
                },
              },
              enabled: true,
              health: tested ? "healthy" : "untested",
              lastCheckedAt: tested ? "2026-05-12T09:25:00.000Z" : null,
            },
          ],
        }),
        {
          headers: {
            "Content-Type": "application/json",
          },
        },
      );
    }

    if (path === "/api/sources/src-local/test" && init?.method === "POST") {
      tested = true;
      return new Response(
        JSON.stringify({
          ok: true,
          checkedAt: "2026-05-12T09:25:00.000Z",
        }),
        {
          headers: {
            "Content-Type": "application/json",
          },
        },
      );
    }

    return new Response(JSON.stringify(responseFor(path)), {
      headers: {
        "Content-Type": "application/json",
      },
    });
  }) as typeof fetch;

  await loadConsoleData();
  expect(get(sources).data[0]?.health).toBe("untested");

  await testSourceConnection("src-local");
  expect(get(sources).data[0]?.health).toBe("healthy");
  expect(get(sources).data[0]?.lastCheckedAt).toBe("2026-05-12T09:25:00.000Z");

  await loadConsoleData();
  expect(get(sources).data[0]?.health).toBe("healthy");
  expect(get(sources).data[0]?.lastCheckedAt).toBe("2026-05-12T09:25:00.000Z");
});

test("api mutating calls reject live business errors instead of using mock fallback", async () => {
  globalThis.fetch = (async (input, init) => {
    const path = requestPath(input);

    if (path === "/api/jobs/job-local/run" && init?.method === "POST") {
      return new Response(
        JSON.stringify({
          error: {
            code: "CONFLICT",
            message: "sync job is already running: job-local",
          },
        }),
        {
          status: 409,
          headers: {
            "Content-Type": "application/json",
          },
        },
      );
    }

    return new Response(JSON.stringify(responseFor(path)), {
      headers: {
        "Content-Type": "application/json",
      },
    });
  }) as typeof fetch;

  try {
    await api.runJob("job-local");
    throw new Error("expected api.runJob to reject");
  } catch (error) {
    expect(error).toMatchObject({
      code: "CONFLICT",
      status: 409,
    });
  }
});

test("api read calls still fall back to mock data when the local endpoint is missing", async () => {
  globalThis.fetch = (async () =>
    new Response("not found", {
      status: 404,
      headers: {
        "Content-Type": "text/plain",
      },
    })) as typeof fetch;

  const result = await api.getSources();

  expect(result.origin).toBe("mock");
  expect(result.error).toMatchObject({
    code: "API_UNAVAILABLE",
    status: 404,
  });
  expect(result.data.length).toBeGreaterThan(0);
});

test("loadFiles requests the file browser endpoint and stores directory entries", async () => {
  const paths: string[] = [];

  globalThis.fetch = (async (input) => {
    const path = requestPathWithSearch(input);
    paths.push(path);

    if (path === "/api/files?sourceId=1&path=docs") {
      return jsonResponse({
        sourceId: 1,
        path: "docs",
        entries: [
          {
            name: "guide.md",
            path: "docs/guide.md",
            kind: "file",
            item: {
              id: 10,
              sourceId: 1,
              sourcePath: "docs/guide.md",
              itemType: "file",
              status: "synced",
              size: 2048,
            },
          },
        ],
      });
    }

    return jsonResponse(responseFor(requestPath(input)));
  }) as typeof fetch;

  await loadFiles({ sourceId: 1, path: "docs" });

  expect(paths).toEqual(["/api/files?sourceId=1&path=docs"]);
  expect(get(fileBrowser).data).toMatchObject({
    sourceId: 1,
    path: "docs",
    entries: [
      {
        name: "guide.md",
        kind: "file",
        item: {
          sourcePath: "docs/guide.md",
          status: "synced",
        },
      },
    ],
  });
});

test("createSource posts app connector config variants", async () => {
  const postBodies: unknown[] = [];

  globalThis.fetch = (async (input, init) => {
    const path = requestPath(input);

    if (path === "/api/sources" && init?.method === "POST") {
      const body = JSON.parse(String(init.body));
      postBodies.push(body);
      return jsonResponse({
        id: postBodies.length,
        name: body.name,
        connectorKind: body.config.kind,
        config: {
          service: body.config.kind,
          options:
            body.config.kind === "notion"
              ? {
                  token: "<redacted>",
                  data_source_id: body.config.dataSourceId,
                  version: body.config.version,
                }
              : {
                  app_id: body.config.appId,
                  app_secret: "<redacted>",
                  folder_token: body.config.folderToken,
                },
        },
        enabled: body.enabled,
        health: "untested",
      });
    }

    return jsonResponse(responseFor(path));
  }) as typeof fetch;

  await api.createSource({
    name: "Notion knowledge",
    serviceKind: "notion",
    enabled: true,
    config: {
      token: "secret-token",
      dataSourceId: "ds_123",
      version: "2026-03-11",
    },
  });
  await api.createSource({
    name: "Feishu drive",
    serviceKind: "feishu",
    enabled: true,
    config: {
      appId: "cli_xxx",
      appSecret: "app-secret",
      folderToken: "folder_123",
    },
  });

  expect(postBodies).toEqual([
    {
      name: "Notion knowledge",
      config: {
        kind: "notion",
        token: "secret-token",
        dataSourceId: "ds_123",
        version: "2026-03-11",
      },
      enabled: true,
    },
    {
      name: "Feishu drive",
      config: {
        kind: "feishu",
        appId: "cli_xxx",
        appSecret: "app-secret",
        folderToken: "folder_123",
      },
      enabled: true,
    },
  ]);
});

test("updateSource patches the live source and refreshes dependent job names", async () => {
  const patchBodies: unknown[] = [];

  globalThis.fetch = (async (input, init) => {
    const path = requestPath(input);

    if (path === "/api/sources/src-local" && init?.method === "PATCH") {
      patchBodies.push(JSON.parse(String(init.body)));
      return jsonResponse({
        id: "src-local",
        name: "Edited source",
        connectorKind: "opendal",
        config: {
          service: "s3",
          options: {
            bucket: "edited-archive",
            region: "auto",
            access_key_id: "<redacted>",
            secret_access_key: "<redacted>",
          },
        },
        enabled: false,
        health: "disabled",
        lastCheckedAt: null,
      });
    }

    return jsonResponse(responseFor(path));
  }) as typeof fetch;

  await loadConsoleData();
  await updateSource("src-local", {
    name: "Edited source",
    serviceKind: "s3",
    enabled: false,
    config: {
      bucket: "edited-archive",
      region: "auto",
      accessKeyId: "<redacted>",
      secretAccessKey: "<redacted>",
    },
  });

  expect(patchBodies).toEqual([
    {
      name: "Edited source",
      config: {
        kind: "opendal",
        service: "s3",
        options: {
          bucket: "edited-archive",
          region: "auto",
          access_key_id: "<redacted>",
          secret_access_key: "<redacted>",
        },
      },
      enabled: false,
    },
  ]);
  expect(get(sources).data[0]).toMatchObject({
    id: "src-local",
    name: "Edited source",
    serviceKind: "s3",
    enabled: false,
    health: "disabled",
  });
  expect(get(sources).data[0]?.config).toMatchObject({
    bucket: "edited-archive",
    region: "auto",
    access_key_id: "<redacted>",
  });
  expect(get(sources).data[0]?.lastCheckedAt).toBeUndefined();
  expect(get(sources).data[0]?.lastError).toBeUndefined();
  expect(get(jobs).data[0]?.sourceName).toBe("Edited source");
});

test("updateJob patches the live sync job and replaces it in state", async () => {
  const patchBodies: unknown[] = [];

  globalThis.fetch = (async (input, init) => {
    const path = requestPath(input);

    if (path === "/api/jobs/job-local" && init?.method === "PATCH") {
      patchBodies.push(JSON.parse(String(init.body)));
      return jsonResponse({
        id: "job-local",
        sourceId: "src-local",
        name: "Edited job",
        enabled: false,
        schedule: { kind: "interval", intervalSeconds: 900 },
        status: "paused",
        nextRunAt: null,
        lastRunAt: null,
        lastRunStatus: null,
        lastRunId: null,
      });
    }

    return jsonResponse(responseFor(path));
  }) as typeof fetch;

  await loadConsoleData();
  await updateJob("job-local", {
    sourceId: "src-local",
    name: "Edited job",
    enabled: false,
    schedule: { kind: "interval", intervalSeconds: 900 },
  });

  expect(patchBodies).toEqual([
    {
      sourceId: "src-local",
      name: "Edited job",
      enabled: false,
      schedule: { kind: "interval", intervalSeconds: 900 },
    },
  ]);
  expect(get(jobs).data[0]).toMatchObject({
    id: "job-local",
    sourceId: "src-local",
    sourceName: "Local source",
    name: "Edited job",
    enabled: false,
    status: "paused",
    schedule: { kind: "interval", intervalSeconds: 900 },
    scheduleLabel: "Every 15 minutes",
  });
});

test("deleteSource deletes the live source and clears dependent console state", async () => {
  const requests: string[] = [];

  globalThis.fetch = (async (input, init) => {
    const path = requestPath(input);
    const pathWithSearch = requestPathWithSearch(input);
    requests.push(`${init?.method ?? "GET"} ${pathWithSearch}`);

    if (pathWithSearch === "/api/files?sourceId=src-local&path=docs") {
      return jsonResponse({
        sourceId: "src-local",
        path: "docs",
        entries: [
          {
            name: "guide.md",
            path: "docs/guide.md",
            kind: "file",
          },
        ],
      });
    }

    if (path === "/api/sources/src-local" && init?.method === "DELETE") {
      return new Response(null, { status: 204 });
    }

    return jsonResponse(responseFor(path));
  }) as typeof fetch;

  await loadConsoleData();
  await loadFiles({ sourceId: "src-local", path: "docs" });

  expect(get(fileBrowser).data?.sourceId).toBe("src-local");

  await deleteSource("src-local");

  expect(requests).toContain("DELETE /api/sources/src-local");
  expect(get(sources).data).toEqual([]);
  expect(get(jobs).data).toEqual([]);
  expect(get(fileBrowser).data).toBeUndefined();
});

test("triggerJobRun keeps refreshed run list data when the live API returns the new run", async () => {
  globalThis.fetch = (async (input, init) => {
    const path = requestPath(input);

    if (path === "/api/jobs/job-local/run" && init?.method === "POST") {
      return new Response(
        JSON.stringify({
          runId: "run-new",
          status: "synced",
        }),
        {
          headers: {
            "Content-Type": "application/json",
          },
        },
      );
    }

    return new Response(JSON.stringify(responseFor(path, { includeNewRun: true })), {
      headers: {
        "Content-Type": "application/json",
      },
    });
  }) as typeof fetch;

  await loadConsoleData();
  await triggerJobRun("job-local");

  const [latestRun] = get(runs).data;
  expect(latestRun.id).toBe("run-new");
  expect(latestRun.startedAt).toBe("2026-05-12T09:30:00.000Z");
  expect(latestRun.counts).toMatchObject({
    processed: 5,
    synced: 4,
    skipped: 1,
    failed: 0,
  });
});

test("triggerJobRun mock fallback does not leave the job running", async () => {
  globalThis.fetch = (async () =>
    new Response("not found", {
      status: 404,
      headers: {
        "Content-Type": "text/plain",
      },
    })) as typeof fetch;

  await triggerJobRun(1);

  const job = get(jobs).data.find((candidate) => candidate.id === 1);
  expect(job?.status).toBe("idle");
  expect(job?.lastRunStatus).toBe("completed");
});

test("stopJob posts to the live stop endpoint and refreshes running job state", async () => {
  const requests: string[] = [];
  let stopped = false;

  globalThis.fetch = (async (input, init) => {
    const path = requestPath(input);
    requests.push(`${init?.method ?? "GET"} ${path}`);

    if (path === "/api/jobs/job-local/stop" && init?.method === "POST") {
      stopped = true;
      return new Response(null, { status: 202 });
    }

    if (path === "/api/jobs") {
      return jsonResponse({
        data: [
          {
            id: "job-local",
            sourceId: "src-local",
            name: "Local job",
            enabled: true,
            status: stopped ? "idle" : "running",
            schedule: "Manual",
            lastRunStatus: stopped ? "cancelled" : "running",
          },
        ],
      });
    }

    if (path === "/api/runs") {
      return jsonResponse({
        data: [
          {
            id: "run-local",
            jobId: "job-local",
            status: stopped ? "cancelled" : "running",
            startedAt: "2026-05-12T09:24:00.000Z",
            finishedAt: stopped ? "2026-05-12T09:25:00.000Z" : null,
            processedCount: 1,
            syncedCount: 0,
            skippedCount: 0,
            failedCount: 0,
          },
        ],
      });
    }

    return jsonResponse(responseFor(path));
  }) as typeof fetch;

  await loadConsoleData();
  expect(get(jobs).data[0]?.status).toBe("running");

  await stopJob("job-local");

  expect(requests).toContain("POST /api/jobs/job-local/stop");
  expect(get(jobs).data[0]).toMatchObject({
    id: "job-local",
    status: "idle",
    lastRunStatus: "cancelled",
  });
  expect(get(runs).data[0]).toMatchObject({
    id: "run-local",
    status: "cancelled",
  });
});

test("stopJob ignores duplicate clicks while a stop request is in flight", async () => {
  let stopRequests = 0;
  let stopped = false;
  let resolveStop: ((response: Response) => void) | undefined;

  globalThis.fetch = (async (input, init) => {
    const path = requestPath(input);

    if (path === "/api/jobs/job-local/stop" && init?.method === "POST") {
      stopRequests += 1;
      return new Promise<Response>((resolve) => {
        resolveStop = (response) => {
          stopped = true;
          resolve(response);
        };
      });
    }

    return jsonResponse(responseFor(path, { runningJob: !stopped }));
  }) as typeof fetch;

  await loadConsoleData();

  const firstStop = stopJob("job-local");
  const secondStop = stopJob("job-local");
  await Promise.resolve();

  expect(stopRequests).toBe(1);
  expect(get(jobActions).stoppingJobIds).toEqual(["job-local"]);

  resolveStop?.(new Response(null, { status: 202 }));
  await firstStop;
  await secondStop;
});

test("loadRunDetail keeps the latest selected run when detail responses resolve out of order", async () => {
  const detailResolvers = new Map<string, (response: Response) => void>();

  globalThis.fetch = (async (input) => {
    const path = requestPath(input);

    if (path === "/api/runs/run-old" || path === "/api/runs/run-new") {
      return new Promise<Response>((resolve) => {
        detailResolvers.set(path, resolve);
      });
    }

    if (path === "/api/items" || path === "/api/errors") {
      return jsonResponse({ data: [] });
    }

    return jsonResponse(responseFor(path));
  }) as typeof fetch;

  await loadConsoleData();

  const oldRequest = loadRunDetail("run-old");
  const newRequest = loadRunDetail("run-new");

  detailResolvers.get("/api/runs/run-new")?.(jsonResponse(runDetailResponse("run-new")));
  await newRequest;
  expect(get(selectedRunDetail).data?.id).toBe("run-new");

  detailResolvers.get("/api/runs/run-old")?.(jsonResponse(runDetailResponse("run-old")));
  await oldRequest;
  expect(get(selectedRunDetail).data?.id).toBe("run-new");
});

test("loadRunDetail clears stale selected run data while the next detail loads", async () => {
  const detailResolvers = new Map<string, (response: Response) => void>();

  globalThis.fetch = (async (input) => {
    const path = requestPath(input);

    if (path === "/api/runs/run-old" || path === "/api/runs/run-new") {
      return new Promise<Response>((resolve) => {
        detailResolvers.set(path, resolve);
      });
    }

    if (path === "/api/items" || path === "/api/errors") {
      return jsonResponse({ data: [] });
    }

    return jsonResponse(responseFor(path));
  }) as typeof fetch;

  await loadConsoleData();

  const oldRequest = loadRunDetail("run-old");
  detailResolvers.get("/api/runs/run-old")?.(jsonResponse(runDetailResponse("run-old")));
  await oldRequest;
  expect(get(selectedRunDetail).data?.id).toBe("run-old");

  const newRequest = loadRunDetail("run-new");
  expect(get(selectedRunDetail).status).toBe("loading");
  expect(get(selectedRunDetail).data).toBeUndefined();

  detailResolvers.get("/api/runs/run-new")?.(jsonResponse(runDetailResponse("run-new")));
  await newRequest;
  expect(get(selectedRunDetail).data?.id).toBe("run-new");
});

test("loadRunDetail ignores stale failures after a newer detail selection", async () => {
  const detailResolvers = new Map<string, (response: Response) => void>();

  globalThis.fetch = (async (input) => {
    const path = requestPath(input);

    if (path === "/api/runs/run-old" || path === "/api/runs/run-new") {
      return new Promise<Response>((resolve) => {
        detailResolvers.set(path, resolve);
      });
    }

    if (path === "/api/items" || path === "/api/errors") {
      return jsonResponse({ data: [] });
    }

    return jsonResponse(responseFor(path));
  }) as typeof fetch;

  await loadConsoleData();

  const oldRequest = loadRunDetail("run-old");
  const newRequest = loadRunDetail("run-new");

  detailResolvers.get("/api/runs/run-new")?.(jsonResponse(runDetailResponse("run-new")));
  await newRequest;

  detailResolvers.get("/api/runs/run-old")?.(
    jsonResponse(
      {
        error: {
          code: "NOT_FOUND",
          message: "run-old missing",
        },
      },
      404,
    ),
  );
  await oldRequest;

  expect(get(selectedRunDetail).data?.id).toBe("run-new");
  expect(get(selectedRunDetail).error).toBeUndefined();
});

test("loadRunDetail records errors for the latest failed detail request", async () => {
  globalThis.fetch = (async (input) => {
    const path = requestPath(input);

    if (path === "/api/runs/run-missing") {
      return jsonResponse(
        {
          error: {
            code: "NOT_FOUND",
            message: "run-missing missing",
          },
        },
        404,
      );
    }

    if (path === "/api/items" || path === "/api/errors") {
      return jsonResponse({ data: [] });
    }

    return jsonResponse(responseFor(path));
  }) as typeof fetch;

  await loadConsoleData();
  await loadRunDetail("run-missing");

  expect(get(selectedRunDetail).error).toMatchObject({
    code: "NOT_FOUND",
    status: 404,
  });
});

function requestPath(input: RequestInfo | URL) {
  if (typeof input === "string") {
    return new URL(input, "http://localhost").pathname;
  }

  if (input instanceof URL) {
    return input.pathname;
  }

  return new URL(input.url).pathname;
}

function requestPathWithSearch(input: RequestInfo | URL) {
  const url =
    typeof input === "string"
      ? new URL(input, "http://localhost")
      : new URL(input instanceof URL ? input.href : input.url);
  return `${url.pathname}${url.search}`;
}

function jsonResponse(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: {
      "Content-Type": "application/json",
    },
  });
}

function runDetailResponse(id: string) {
  return {
    id,
    jobId: "job-local",
    sourceId: "src-local",
    sourceName: "Local source",
    jobName: "Local job",
    status: "completed",
    startedAt: "2026-05-12T09:30:00.000Z",
    finishedAt: "2026-05-12T09:30:01.000Z",
    durationMs: 1000,
    counts: {
      processed: 1,
      synced: 1,
      skipped: 0,
      failed: 0,
      deleted: 0,
    },
    errors: [],
  };
}

function responseFor(
  path: string,
  options: { includeNewRun?: boolean; runningJob?: boolean } = {},
) {
  switch (path) {
    case "/api/sources":
      return {
        data: [
          {
            id: "src-local",
            name: "Local source",
            connectorKind: "opendal",
            config: {
              service: "fs",
              options: {
                root: "/tmp/hoarder",
              },
            },
            enabled: true,
          },
        ],
      };
    case "/api/jobs":
      return {
        data: [
          {
            id: "job-local",
            sourceId: "src-local",
            name: "Local job",
            enabled: true,
            status: options.runningJob ? "running" : "idle",
            schedule: "Manual",
          },
        ],
      };
    case "/api/runs":
      return {
        data: [
          ...(options.includeNewRun
            ? [
                {
                  id: "run-new",
                  jobId: "job-local",
                  status: "synced",
                  startedAt: "2026-05-12T09:30:00.000Z",
                  finishedAt: "2026-05-12T09:30:02.000Z",
                  processedCount: 5,
                  syncedCount: 4,
                  skippedCount: 1,
                  failedCount: 0,
                },
              ]
            : []),
          {
            id: "run-local",
            jobId: "job-local",
            status: "synced",
            startedAt: "2026-05-12T09:24:00.000Z",
            finishedAt: "2026-05-12T09:24:01.000Z",
            processedCount: 1,
            syncedCount: 1,
            skippedCount: 0,
            failedCount: 0,
          },
        ],
      };
    case "/api/settings":
      return {
        vaultPath: "/tmp/hoarder-vault",
        databasePath: "/tmp/hoarder.sqlite",
        listenAddr: "127.0.0.1:4761",
        jobConcurrency: 1,
        fileConcurrency: 4,
      };
    default:
      throw new Error(`Unexpected request path: ${path}`);
  }
}
