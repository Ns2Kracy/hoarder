import type { OpenDalServiceKind, SourceTemplate } from "./types";

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

export const sourceTemplates: SourceTemplate[] = [
  {
    id: "local-filesystem",
    label: "Local filesystem",
    description: "Sync a local directory into the vault.",
    serviceKind: "fs",
    defaultConfig: {},
  },
  {
    id: "synology-webdav",
    label: "Synology WebDAV",
    description: "Synology Drive or WebDAV Server with an app password.",
    serviceKind: "webdav",
    defaultConfig: {},
  },
  {
    id: "qnap-sftp",
    label: "QNAP SFTP",
    description: "NAS share over SSH/SFTP with key-based auth.",
    serviceKind: "sftp",
    defaultConfig: {},
  },
  {
    id: "minio-s3",
    label: "MinIO / S3-compatible NAS",
    description: "S3-compatible object storage hosted by a NAS or local service.",
    serviceKind: "s3",
    defaultConfig: {
      region: "us-east-1",
    },
  },
  {
    id: "cloudflare-r2",
    label: "Cloudflare R2",
    description: "Cloudflare R2 through its S3-compatible API.",
    serviceKind: "s3",
    defaultConfig: {
      region: "auto",
    },
  },
];

export function sourceTemplateById(templateId: string) {
  return sourceTemplates.find((template) => template.id === templateId);
}

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
