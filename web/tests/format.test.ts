import { expect, test } from "bun:test";
import { formatBytes, formatDuration } from "../src/lib/format";

test("formatDuration renders zero milliseconds as 0s", () => {
  expect(formatDuration(0)).toBe("0s");
});

test("formatDuration renders missing durations as Running", () => {
  expect(formatDuration(undefined)).toBe("Running");
});

test("formatBytes renders compact binary units", () => {
  expect(formatBytes(undefined)).toBe("-");
  expect(formatBytes(512)).toBe("512 B");
  expect(formatBytes(2048)).toBe("2.00 KB");
  expect(formatBytes(2_400_000)).toBe("2.29 MB");
});
