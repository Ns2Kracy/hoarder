<script lang="ts">
  import StatusBadge from "./StatusBadge.svelte";
  import { formatCount, formatDateTime, formatDuration } from "../lib/format";
  import type { LocalId, SyncRunDto } from "../lib/types";

  let {
    runs,
    selectedRunId,
    onSelect
  }: {
    runs: SyncRunDto[];
    selectedRunId?: LocalId;
    onSelect?: (run: SyncRunDto) => void;
  } = $props();
</script>

<div class="overflow-x-auto border-t border-line">
  <table class="min-w-full border-collapse text-left text-sm">
    <thead class="bg-panel-muted text-xs text-subtle">
      <tr>
        <th class="whitespace-nowrap px-3 py-2 font-bold">Run</th>
        <th class="whitespace-nowrap px-3 py-2 font-bold">Source</th>
        <th class="whitespace-nowrap px-3 py-2 font-bold">Status</th>
        <th class="whitespace-nowrap px-3 py-2 font-bold">Started</th>
        <th class="whitespace-nowrap px-3 py-2 text-right font-bold">Synced</th>
        <th class="whitespace-nowrap px-3 py-2 text-right font-bold">Skipped</th>
        <th class="whitespace-nowrap px-3 py-2 text-right font-bold">Failed</th>
        <th class="whitespace-nowrap px-3 py-2 text-right font-bold">Deleted</th>
        <th class="whitespace-nowrap px-3 py-2 font-bold">Duration</th>
      </tr>
    </thead>
    <tbody>
      {#each runs as run (run.id)}
        <tr class={`transition-colors hover:bg-panel-muted ${selectedRunId === run.id ? "bg-accent-soft" : ""}`}>
          <td class="max-w-48 border-t border-line-soft px-3 py-2 align-top font-mono text-xs text-muted">
            <button
              class="max-w-full truncate text-left underline-offset-2 hover:underline"
              type="button"
              onclick={() => onSelect?.(run)}
            >
              {run.id}
            </button>
          </td>
          <td class="border-t border-line-soft px-3 py-2 align-top font-medium text-ink">{run.sourceName}</td>
          <td class="border-t border-line-soft px-3 py-2 align-top"><StatusBadge status={run.status} /></td>
          <td class="whitespace-nowrap border-t border-line-soft px-3 py-2 align-top text-muted">{formatDateTime(run.startedAt)}</td>
          <td class="border-t border-line-soft px-3 py-2 text-right align-top tabular-nums text-emerald-700 dark:text-emerald-300">{formatCount(run.counts.synced)}</td>
          <td class="border-t border-line-soft px-3 py-2 text-right align-top tabular-nums text-muted">{formatCount(run.counts.skipped)}</td>
          <td class="border-t border-line-soft px-3 py-2 text-right align-top tabular-nums text-rose-700 dark:text-rose-300">{formatCount(run.counts.failed)}</td>
          <td class="border-t border-line-soft px-3 py-2 text-right align-top tabular-nums text-orange-700 dark:text-orange-300">{formatCount(run.counts.deleted)}</td>
          <td class="whitespace-nowrap border-t border-line-soft px-3 py-2 align-top text-muted">{formatDuration(run.durationMs)}</td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>
