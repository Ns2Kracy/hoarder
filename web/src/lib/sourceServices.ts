import type { OpenDalServiceKind } from "./types";

export const sourceServiceOptions = [
  { value: "fs", label: "Filesystem", implemented: true },
  { value: "s3", label: "S3", implemented: true },
  { value: "webdav", label: "WebDAV", implemented: true },
  { value: "sftp", label: "SFTP", implemented: true },
] satisfies {
  value: OpenDalServiceKind;
  label: string;
  implemented: boolean;
}[];

export function canSubmitSourceForm(input: {
  name: string;
  serviceKind: OpenDalServiceKind;
  root: string;
  endpoint?: string;
  bucket?: string;
  region?: string;
  username?: string;
  accessKeyId?: string;
  secretAccessKey?: string;
}) {
  if (input.name.trim().length === 0) {
    return false;
  }

  switch (input.serviceKind) {
    case "fs":
      return input.root.trim().length > 0;
    case "webdav":
      return (input.endpoint ?? "").trim().length > 0;
    case "sftp":
      return (input.endpoint ?? "").trim().length > 0 && (input.username ?? "").trim().length > 0;
    case "s3":
      return (
        (input.bucket ?? "").trim().length > 0 &&
        (input.region ?? "").trim().length > 0 &&
        (input.accessKeyId ?? "").trim().length > 0 &&
        (input.secretAccessKey ?? "").trim().length > 0
      );
  }
}
