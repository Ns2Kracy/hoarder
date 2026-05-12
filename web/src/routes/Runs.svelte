<script lang="ts">
  import { AlertTriangle, ListChecks } from "lucide-svelte";
  import FallbackNotice from "../components/FallbackNotice.svelte";
  import RunSummaryTable from "../components/RunSummaryTable.svelte";
  import StatusBadge from "../components/StatusBadge.svelte";
  import { formatDateTime, formatDuration } from "../lib/format";
  import type { Loadable, SyncRunDto } from "../lib/types";

  let { runs }: { runs: Loadable<SyncRunDto[]> } = $props();

  let selectedRunId = $state<string | undefined>(undefined);
  let selectedRun = $derived(runs.data.find((run) => run.id === selectedRunId) ?? runs.data[0]);
</script>

<section class="space-y-4">
  <div>
    <h1 class="text-xl font-semibold text-zinc-950">Runs</h1>
    <p class="mt-1 text-sm text-zinc-600">Review sync outcomes, item counts, and structured errors.</p>
  </div>

  <section class="rounded border border-zinc-200 bg-white">
    <div class="flex items-center gap-2 border-b border-zinc-200 px-3 py-2">
      <ListChecks aria-hidden="true" size={16} class="text-zinc-500" />
      <h2 class="text-sm font-semibold text-zinc-900">Run History</h2>
    </div>

    {#if runs.data.length === 0}
      <div class="px-3 py-8 text-sm text-zinc-500" role="status">No sync runs have been recorded yet.</div>
    {:else}
      <RunSummaryTable runs={runs.data} selectedRunId={selectedRun?.id} onSelect={(run) => (selectedRunId = run.id)} />
    {/if}
  </section>

  {#if selectedRun}
    <section class="rounded border border-zinc-200 bg-white">
      <div class="flex flex-wrap items-center justify-between gap-2 border-b border-zinc-200 px-3 py-2">
        <div>
          <h2 class="text-sm font-semibold text-zinc-900">{selectedRun.sourceName}</h2>
          <p class="font-mono text-xs text-zinc-500">{selectedRun.id}</p>
        </div>
        <StatusBadge status={selectedRun.status} />
      </div>

      <div class="grid gap-3 p-3 md:grid-cols-5">
        <div>
          <p class="text-xs text-zinc-500">Processed</p>
          <p class="text-lg font-semibold tabular-nums text-zinc-950">{selectedRun.counts.processed}</p>
        </div>
        <div>
          <p class="text-xs text-zinc-500">Synced</p>
          <p class="text-lg font-semibold tabular-nums text-emerald-700">{selectedRun.counts.synced}</p>
        </div>
        <div>
          <p class="text-xs text-zinc-500">Skipped</p>
          <p class="text-lg font-semibold tabular-nums text-zinc-700">{selectedRun.counts.skipped}</p>
        </div>
        <div>
          <p class="text-xs text-zinc-500">Failed</p>
          <p class="text-lg font-semibold tabular-nums text-rose-700">{selectedRun.counts.failed}</p>
        </div>
        <div>
          <p class="text-xs text-zinc-500">Deleted on source</p>
          <p class="text-lg font-semibold tabular-nums text-orange-700">{selectedRun.counts.deleted}</p>
        </div>
      </div>

      <div class="grid gap-3 border-t border-zinc-200 p-3 md:grid-cols-3">
        <div>
          <p class="text-xs text-zinc-500">Started</p>
          <p class="text-sm text-zinc-900">{formatDateTime(selectedRun.startedAt)}</p>
        </div>
        <div>
          <p class="text-xs text-zinc-500">Finished</p>
          <p class="text-sm text-zinc-900">{formatDateTime(selectedRun.finishedAt)}</p>
        </div>
        <div>
          <p class="text-xs text-zinc-500">Duration</p>
          <p class="text-sm text-zinc-900">{formatDuration(selectedRun.durationMs)}</p>
        </div>
      </div>

      {#if selectedRun.errors.length > 0}
        <div class="border-t border-zinc-200 p-3">
          <div class="mb-2 flex items-center gap-2">
            <AlertTriangle aria-hidden="true" size={16} class="text-rose-600" />
            <h3 class="text-sm font-semibold text-zinc-900">Errors</h3>
          </div>
          <div class="space-y-2">
            {#each selectedRun.errors as error (error.id)}
              <div class="rounded border border-rose-200 bg-rose-50 p-2">
                <div class="flex flex-wrap items-center justify-between gap-2">
                  <p class="font-mono text-xs font-semibold text-rose-950">{error.code}</p>
                  <p class="text-xs text-rose-800">{formatDateTime(error.createdAt)}</p>
                </div>
                <p class="mt-1 text-sm text-rose-950">{error.message}</p>
                {#if error.sourcePath}
                  <p class="mt-1 truncate font-mono text-xs text-rose-900">{error.sourcePath}</p>
                {/if}
                <pre class="mt-2 max-h-44 overflow-auto rounded border border-rose-200 bg-white p-2 font-mono text-xs text-rose-950">{JSON.stringify(error.details ?? {}, null, 2)}</pre>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </section>
  {/if}

  <FallbackNotice error={runs.error} />
</section>
