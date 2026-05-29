import { expect, test } from "bun:test";
import {
  canSubmitSourceForm,
  sourceServiceOptions,
  sourceTemplateById,
  sourceTemplates,
} from "../src/lib/sourceServices";

test("all OpenDAL storage source services are selectable", () => {
  expect(
    sourceServiceOptions.filter((option) => option.implemented).map((option) => option.value),
  ).toEqual(["fs", "s3", "webdav", "sftp"]);
  expect(
    sourceServiceOptions.filter((option) => !option.implemented).map((option) => option.value),
  ).toEqual([]);
});

test("source form submission validates required service fields", () => {
  expect(canSubmitSourceForm({ name: "Docs", serviceKind: "fs", root: "/tmp/docs" })).toBe(true);
  expect(
    canSubmitSourceForm({
      name: "Docs",
      serviceKind: "webdav",
      root: "",
      endpoint: "https://dav.example.test",
    }),
  ).toBe(true);
  expect(
    canSubmitSourceForm({
      name: "Docs",
      serviceKind: "sftp",
      root: "",
      endpoint: "ssh://example.test",
      username: "ada",
    }),
  ).toBe(true);
  expect(
    canSubmitSourceForm({
      name: "Docs",
      serviceKind: "s3",
      root: "",
      bucket: "archive",
      region: "us-east-1",
      accessKeyId: "access",
      secretAccessKey: "secret",
    }),
  ).toBe(true);

  expect(canSubmitSourceForm({ name: "Docs", serviceKind: "s3", root: "/tmp/docs" })).toBe(false);
  expect(canSubmitSourceForm({ name: "Docs", serviceKind: "fs", root: "" })).toBe(false);
});

test("source templates include NAS and S3-compatible presets", () => {
  expect(sourceTemplates.map((template) => template.id)).toEqual([
    "local-filesystem",
    "synology-webdav",
    "qnap-sftp",
    "minio-s3",
    "cloudflare-r2",
  ]);
  expect(sourceTemplateById("synology-webdav")?.serviceKind).toBe("webdav");
  expect(sourceTemplateById("qnap-sftp")?.serviceKind).toBe("sftp");
  expect(sourceTemplateById("minio-s3")?.defaultConfig.region).toBe("us-east-1");
});
