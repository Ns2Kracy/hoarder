<script lang="ts">
  import { AlertTriangle, Ban, CheckCircle2, CircleDashed, Clock3, PlayCircle, XCircle } from "lucide-svelte";
  import type { ItemSyncStatus, JobStatus, RunStatus, SourceHealth } from "../lib/types";

  type Status = SourceHealth | JobStatus | RunStatus | ItemSyncStatus;

  const labelByStatus: Record<Status, string> = {
    healthy: "Healthy",
    warning: "Warning",
    failed: "Failed",
    untested: "Untested",
    disabled: "Disabled",
    idle: "Idle",
    running: "Running",
    paused: "Paused",
    completed: "Completed",
    completed_with_failures: "Completed with failures",
    cancelled: "Cancelled",
    pending: "Pending",
    synced: "Synced",
    skipped: "Skipped",
    deleted_on_source: "Deleted on source"
  };

  const classByStatus: Record<Status, string> = {
    healthy: "border-emerald-300/60 bg-emerald-500/10 text-emerald-800 dark:text-emerald-200",
    warning: "border-amber-300/60 bg-amber-500/10 text-amber-800 dark:text-amber-200",
    failed: "border-rose-300/60 bg-rose-500/10 text-rose-800 dark:text-rose-200",
    untested: "border-line bg-panel-muted text-muted",
    disabled: "border-line bg-panel-muted text-muted",
    idle: "border-sky-300/60 bg-sky-500/10 text-sky-800 dark:text-sky-200",
    running: "border-accent-border bg-accent-soft text-blue-800 motion-safe:[&_svg]:animate-status-pulse dark:text-blue-100",
    paused: "border-line bg-panel-muted text-muted",
    completed: "border-emerald-300/60 bg-emerald-500/10 text-emerald-800 dark:text-emerald-200",
    completed_with_failures: "border-amber-300/60 bg-amber-500/10 text-amber-800 dark:text-amber-200",
    cancelled: "border-line bg-panel-muted text-muted",
    pending: "border-sky-300/60 bg-sky-500/10 text-sky-800 dark:text-sky-200",
    synced: "border-emerald-300/60 bg-emerald-500/10 text-emerald-800 dark:text-emerald-200",
    skipped: "border-line bg-panel-muted text-muted",
    deleted_on_source: "border-orange-300/60 bg-orange-500/10 text-orange-800 dark:text-orange-200"
  };

  const iconByStatus = {
    healthy: CheckCircle2,
    warning: AlertTriangle,
    failed: XCircle,
    untested: CircleDashed,
    disabled: Ban,
    idle: Clock3,
    running: PlayCircle,
    paused: Ban,
    completed: CheckCircle2,
    completed_with_failures: AlertTriangle,
    cancelled: Ban,
    pending: Clock3,
    synced: CheckCircle2,
    skipped: CircleDashed,
    deleted_on_source: AlertTriangle
  } satisfies Record<Status, typeof CheckCircle2>;

  let { status, label }: { status: Status; label?: string } = $props();
  let displayLabel = $derived(label ?? labelByStatus[status]);
  let Icon = $derived(iconByStatus[status]);
</script>

<span
  class={`inline-flex min-w-0 items-center gap-1 rounded-sm border px-1.5 py-0.5 text-xs font-bold leading-4 ${classByStatus[status]}`}
>
  <Icon aria-hidden="true" size={12} strokeWidth={2.2} />
  <span class="truncate">{displayLabel}</span>
</span>
