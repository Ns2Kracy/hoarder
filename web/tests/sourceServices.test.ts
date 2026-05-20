import { expect, test } from "bun:test";
import { canSubmitSourceForm, sourceServiceOptions } from "../src/lib/sourceServices";

test("only filesystem source service is currently selectable", () => {
  expect(
    sourceServiceOptions.filter((option) => option.implemented).map((option) => option.value),
  ).toEqual(["fs"]);
  expect(
    sourceServiceOptions.filter((option) => !option.implemented).map((option) => option.value),
  ).toEqual(["s3", "webdav", "sftp"]);
});

test("source form submission is only valid for filesystem roots", () => {
  expect(canSubmitSourceForm({ name: "Docs", serviceKind: "fs", root: "/tmp/docs" })).toBe(true);
  expect(canSubmitSourceForm({ name: "Docs", serviceKind: "s3", root: "/tmp/docs" })).toBe(false);
  expect(canSubmitSourceForm({ name: "Docs", serviceKind: "fs", root: "" })).toBe(false);
});
