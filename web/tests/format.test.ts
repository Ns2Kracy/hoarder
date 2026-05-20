import { expect, test } from "bun:test";
import { formatDuration } from "../src/lib/format";

test("formatDuration renders zero milliseconds as 0s", () => {
  expect(formatDuration(0)).toBe("0s");
});

test("formatDuration renders missing durations as Running", () => {
  expect(formatDuration(undefined)).toBe("Running");
});
