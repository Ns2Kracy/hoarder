<script lang="ts">
  import { AlertTriangle, Archive, Database, PlayCircle, RefreshCcw } from "lucide-svelte";
  import FallbackNotice from "../components/FallbackNotice.svelte";
  import StatusBadge from "../components/StatusBadge.svelte";
  import RunSummaryTable from "../components/RunSummaryTable.svelte";
  import { formatCount, formatDateTime } from "../lib/format";
  import type { ConsoleSummary, Loadable, SourceDto, SyncJobDto, SyncRunDto } from "../lib/types";

  let {
    summary,
    sources,
    jobs,
    runs,
    onRefresh
  }: {
    summary: ConsoleSummary;
    sources: Loadable<SourceDto[]>;
    jobs: Loadable<SyncJobDto[]>;
    runs: Loadable<SyncRunDto[]>;
    onRefresh: () => void;
  } = $props();

  let recentRuns = $derived(runs.data.slice(0, 5));
</script>

<section class="space-y-4">
  <div class="flex flex-wrap items-start justify-between gap-3">
    <div>
      <h1 class="text-xl font-semibold text-zinc-950">Overview</h1>
      <p class="mt-1 text-sm text-zinc-600">Local connector sync status and recent activity.</p>
    </div>
    <button
      class="inline-flex h-8 items-center gap-1 rounded-sm border border-zinc-300 bg-white px-3 text-sm font-medium text-zinc-800 hover:bg-zinc-50"
      type="button"
      onclick={onRefresh}
    >
      <RefreshCcw aria-hidden="true" size={15} />
      Refresh
    </button>
  </div>

  <div class="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
    <div class="rounded-sm border border-zinc-200 bg-white p-3">
      <div class="flex items-center justify-between gap-2">
        <p class="text-xs font-medium uppercase tracking-normal text-zinc-500">Sources</p>
        <Database aria-hidden="true" class="text-zinc-500" size={17} />
      </div>
      <p class="mt-2 text-2xl font-semibold tabular-nums text-zinc-950">{summary.enabledSourceCount}/{summary.sourceCount}</p>
      <p class="mt-1 text-xs text-zinc-500">Enabled connectors</p>
    </div>

    <div class="rounded-sm border border-zinc-200 bg-white p-3">
      <div class="flex items-center justify-between gap-2">
        <p class="text-xs font-medium uppercase tracking-normal text-zinc-500">Active Jobs</p>
        <PlayCircle aria-hidden="true" class="text-zinc-500" size={17} />
      </div>
      <p class="mt-2 text-2xl font-semibold tabular-nums text-zinc-950">{summary.runningJobCount}/{summary.activeJobCount}</p>
      <p class="mt-1 text-xs text-zinc-500">Running now / enabled</p>
    </div>

    <div class="rounded-sm border border-zinc-200 bg-white p-3">
      <div class="flex items-center justify-between gap-2">
        <p class="text-xs font-medium uppercase tracking-normal text-zinc-500">Failed Items</p>
        <AlertTriangle aria-hidden="true" class="text-zinc-500" size={17} />
      </div>
      <p class="mt-2 text-2xl font-semibold tabular-nums text-rose-700">{formatCount(summary.failedItemCount)}</p>
      <p class="mt-1 text-xs text-zinc-500">Across visible runs</p>
    </div>

    <div class="rounded-sm border border-zinc-200 bg-white p-3">
      <div class="flex items-center justify-between gap-2">
        <p class="text-xs font-medium uppercase tracking-normal text-zinc-500">Vault Size</p>
        <Archive aria-hidden="true" class="text-zinc-500" size={17} />
      </div>
      <p class="mt-2 text-2xl font-semibold tabular-nums text-zinc-950">{summary.vaultSizeLabel}</p>
      <p class="mt-1 text-xs text-zinc-500">Estimated from synced items</p>
    </div>
  </div>

  <div class="grid gap-4 xl:grid-cols-[1.15fr_0.85fr]">
    <section class="rounded-sm border border-zinc-200 bg-white">
      <div class="flex items-center justify-between gap-3 border-b border-zinc-200 px-3 py-2">
        <h2 class="text-sm font-semibold text-zinc-900">Recent Runs</h2>
        {#if summary.lastRun}
          <span class="text-xs text-zinc-500">Last started {formatDateTime(summary.lastRun.startedAt)}</span>
        {/if}
      </div>

      {#if recentRuns.length > 0}
        <RunSummaryTable runs={recentRuns} selectedRunId={summary.lastRun?.id} />
      {:else}
        <div class="px-3 py-8 text-sm text-zinc-500" role="status">No sync runs have been recorded yet.</div>
      {/if}
    </section>

    <section class="rounded-sm border border-zinc-200 bg-white">
      <div class="border-b border-zinc-200 px-3 py-2">
        <h2 class="text-sm font-semibold text-zinc-900">Source Health</h2>
      </div>
      <div class="divide-y divide-zinc-100">
        {#each sources.data as source (source.id)}
          <div class="flex items-center justify-between gap-3 px-3 py-2">
            <div class="min-w-0">
              <p class="truncate text-sm font-medium text-zinc-900">{source.name}</p>
              <p class="truncate text-xs text-zinc-500">{source.serviceKind} · {formatCount(source.itemCount)} items</p>
            </div>
            <StatusBadge status={source.health} />
          </div>
        {/each}
      </div>
    </section>
  </div>

  <FallbackNotice error={sources.error ?? jobs.error ?? runs.error} />
</section>
