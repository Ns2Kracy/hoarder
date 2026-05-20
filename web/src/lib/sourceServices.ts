import type { OpenDalServiceKind } from "./types";

export const sourceServiceOptions = [
  { value: "fs", label: "Filesystem", implemented: true },
  { value: "s3", label: "S3", implemented: false },
  { value: "webdav", label: "WebDAV", implemented: false },
  { value: "sftp", label: "SFTP", implemented: false },
] satisfies {
  value: OpenDalServiceKind;
  label: string;
  implemented: boolean;
}[];

export function canSubmitSourceForm(input: {
  name: string;
  serviceKind: OpenDalServiceKind;
  root: string;
}) {
  return input.name.trim().length > 0 && input.serviceKind === "fs" && input.root.trim().length > 0;
}
