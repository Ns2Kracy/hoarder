<script lang="ts">
  import StatusBadge from "./StatusBadge.svelte";
  import { formatCount, formatDateTime, formatDuration } from "../lib/format";
  import type { SyncRunDto } from "../lib/types";

  let {
    runs,
    selectedRunId,
    onSelect
  }: {
    runs: SyncRunDto[];
    selectedRunId?: string;
    onSelect?: (run: SyncRunDto) => void;
  } = $props();
</script>

<div class="overflow-x-auto border-t border-zinc-200">
  <table class="min-w-full divide-y divide-zinc-200 text-left text-sm">
    <thead class="bg-zinc-50 text-xs uppercase tracking-normal text-zinc-500">
      <tr>
        <th class="px-3 py-2 font-semibold">Run</th>
        <th class="px-3 py-2 font-semibold">Source</th>
        <th class="px-3 py-2 font-semibold">Status</th>
        <th class="px-3 py-2 font-semibold">Started</th>
        <th class="px-3 py-2 text-right font-semibold">Synced</th>
        <th class="px-3 py-2 text-right font-semibold">Skipped</th>
        <th class="px-3 py-2 text-right font-semibold">Failed</th>
        <th class="px-3 py-2 text-right font-semibold">Deleted</th>
        <th class="px-3 py-2 font-semibold">Duration</th>
      </tr>
    </thead>
    <tbody class="divide-y divide-zinc-100 bg-white">
      {#each runs as run (run.id)}
        <tr class={`hover:bg-zinc-50 ${selectedRunId === run.id ? "bg-blue-50/60" : ""}`}>
          <td class="max-w-48 px-3 py-2 font-mono text-xs text-zinc-700">
            <button
              class="max-w-full truncate text-left underline-offset-2 hover:underline"
              type="button"
              onclick={() => onSelect?.(run)}
            >
              {run.id}
            </button>
          </td>
          <td class="px-3 py-2 font-medium text-zinc-900">{run.sourceName}</td>
          <td class="px-3 py-2"><StatusBadge status={run.status} /></td>
          <td class="whitespace-nowrap px-3 py-2 text-zinc-600">{formatDateTime(run.startedAt)}</td>
          <td class="px-3 py-2 text-right tabular-nums text-emerald-700">{formatCount(run.counts.synced)}</td>
          <td class="px-3 py-2 text-right tabular-nums text-zinc-600">{formatCount(run.counts.skipped)}</td>
          <td class="px-3 py-2 text-right tabular-nums text-rose-700">{formatCount(run.counts.failed)}</td>
          <td class="px-3 py-2 text-right tabular-nums text-orange-700">{formatCount(run.counts.deleted)}</td>
          <td class="whitespace-nowrap px-3 py-2 text-zinc-600">{formatDuration(run.durationMs)}</td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>
