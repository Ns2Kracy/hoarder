import { afterEach, expect, test } from "bun:test";
import { loadConsoleData } from "../src/lib/state";

const originalFetch = globalThis.fetch;

afterEach(() => {
  globalThis.fetch = originalFetch;
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

function requestPath(input: RequestInfo | URL) {
  if (typeof input === "string") {
    return new URL(input, "http://localhost").pathname;
  }

  if (input instanceof URL) {
    return input.pathname;
  }

  return new URL(input.url).pathname;
}

function responseFor(path: string) {
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
            schedule: "Manual",
          },
        ],
      };
    case "/api/runs":
      return {
        data: [
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
