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
    scheduled: "Scheduled",
    running: "Running",
    paused: "Paused",
    completed: "Completed",
    cancelled: "Cancelled",
    pending: "Pending",
    synced: "Synced",
    skipped: "Skipped",
    deleted_on_source: "Deleted on source"
  };

  const classByStatus: Record<Status, string> = {
    healthy: "border-emerald-200 bg-emerald-50 text-emerald-800",
    warning: "border-amber-200 bg-amber-50 text-amber-800",
    failed: "border-rose-200 bg-rose-50 text-rose-800",
    untested: "border-zinc-200 bg-zinc-50 text-zinc-700",
    disabled: "border-zinc-200 bg-zinc-100 text-zinc-600",
    scheduled: "border-sky-200 bg-sky-50 text-sky-800",
    running: "border-blue-200 bg-blue-50 text-blue-800",
    paused: "border-zinc-200 bg-zinc-100 text-zinc-600",
    completed: "border-emerald-200 bg-emerald-50 text-emerald-800",
    cancelled: "border-zinc-200 bg-zinc-100 text-zinc-600",
    pending: "border-sky-200 bg-sky-50 text-sky-800",
    synced: "border-emerald-200 bg-emerald-50 text-emerald-800",
    skipped: "border-zinc-200 bg-zinc-50 text-zinc-700",
    deleted_on_source: "border-orange-200 bg-orange-50 text-orange-800"
  };

  const iconByStatus = {
    healthy: CheckCircle2,
    warning: AlertTriangle,
    failed: XCircle,
    untested: CircleDashed,
    disabled: Ban,
    scheduled: Clock3,
    running: PlayCircle,
    paused: Ban,
    completed: CheckCircle2,
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
  class={`inline-flex min-w-0 items-center gap-1 rounded-sm border px-1.5 py-0.5 text-xs font-medium leading-4 ${classByStatus[status]}`}
>
  <Icon aria-hidden="true" size={12} strokeWidth={2.2} />
  <span class="truncate">{displayLabel}</span>
</span>
