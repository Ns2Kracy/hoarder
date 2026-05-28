<script lang="ts">
    import {
        Activity,
        AlertTriangle,
        Archive,
        Database,
        Gauge,
        PlayCircle,
        RefreshCcw,
    } from "lucide-svelte";
    import FallbackNotice from "../components/FallbackNotice.svelte";
    import RunTrendChart from "../components/RunTrendChart.svelte";
    import StatusBadge from "../components/StatusBadge.svelte";
    import RunSummaryTable from "../components/RunSummaryTable.svelte";
    import { formatCount, formatDateTime } from "../lib/format";
    import type {
        ConsoleSummary,
        Loadable,
        SourceDto,
        SyncJobDto,
        SyncRunDto,
    } from "../lib/types";

    let {
        summary,
        sources,
        jobs,
        runs,
        onRefresh,
    }: {
        summary: ConsoleSummary;
        sources: Loadable<SourceDto[]>;
        jobs: Loadable<SyncJobDto[]>;
        runs: Loadable<SyncRunDto[]>;
        onRefresh: () => void;
    } = $props();

    let recentRuns = $derived(runs.data.slice(0, 5));
    let trackedJobs = $derived(jobs.data.slice(0, 4));
    let attentionSources = $derived(
        sources.data
            .filter((source) => needsAttention(source))
            .slice(0, 3),
    );
    let normalSources = $derived(
        sources.data
            .filter((source) => !needsAttention(source))
            .slice(0, 5),
    );
    let activeSourceLabel = $derived(
        `${formatCount(summary.enabledSourceCount)} of ${formatCount(summary.sourceCount)}`,
    );
    let lastRunLabel = $derived(
        summary.lastRun
            ? formatDateTime(summary.lastRun.startedAt)
            : "No runs recorded",
    );

    function needsAttention(source: SourceDto) {
        return (
            source.health === "failed" ||
            source.health === "warning" ||
            source.health === "disabled"
        );
    }
</script>

<section class="grid gap-3 motion-safe:animate-[cockpit-enter_380ms_cubic-bezier(0.16,1,0.3,1)_both]">
    <header class="grid gap-3 rounded-sm border border-line bg-panel-strong p-3 shadow-panel lg:grid-cols-[minmax(0,1fr)_auto] lg:items-end">
        <div class="min-w-0">
            <h1 class="max-w-[13ch] text-[clamp(2rem,4vw,3.8rem)] font-bold leading-[0.96] text-ink">
                Local sync control
            </h1>
            <p class="mt-2 max-w-[54ch] text-sm leading-snug text-muted">
                Connect sources, run jobs, and audit recent activity from this local console.
            </p>
        </div>

        <div class="flex min-w-0 flex-wrap items-center gap-2 lg:justify-end">
            {#if summary.lastRun}
                <StatusBadge status={summary.lastRun.status} />
            {/if}
            <p class="max-w-56 truncate text-xs text-subtle">
                Last run {lastRunLabel}
            </p>
            <button
                class="inline-flex h-9 min-w-max items-center justify-center gap-1 rounded-sm border border-ink bg-ink px-3 text-sm font-semibold text-panel-strong transition hover:border-accent hover:bg-accent hover:text-white active:translate-y-px"
                type="button"
                onclick={onRefresh}
            >
                <RefreshCcw aria-hidden="true" size={15} />
                Refresh
            </button>
        </div>
    </header>

    <div class="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
        <div class="relative grid min-h-32 content-between overflow-hidden rounded-sm border border-line bg-panel-strong p-3 shadow-panel motion-safe:animate-[cockpit-enter_380ms_cubic-bezier(0.16,1,0.3,1)_both]">
            <div class="flex items-center justify-between gap-2">
                <p class="text-xs font-semibold text-subtle">Sources</p>
                <Database aria-hidden="true" class="text-subtle" size={17} />
            </div>
            <div>
                <p class="text-[clamp(1.85rem,2.8vw,2.65rem)] font-bold leading-none tabular-nums text-ink">
                    {activeSourceLabel}
                </p>
                <p class="mt-1 text-xs text-subtle">Enabled connectors</p>
            </div>
            <div class="absolute inset-x-0 bottom-0 h-0.5 bg-[linear-gradient(90deg,var(--accent),transparent)]"></div>
        </div>

        <div class="relative grid min-h-32 content-between overflow-hidden rounded-sm border border-line bg-panel-strong p-3 shadow-panel motion-safe:animate-[cockpit-enter_380ms_cubic-bezier(0.16,1,0.3,1)_both] motion-safe:[animation-delay:40ms]">
            <div class="flex items-center justify-between gap-2">
                <p class="text-xs font-semibold text-subtle">Jobs</p>
                <PlayCircle aria-hidden="true" class="text-subtle" size={17} />
            </div>
            <div>
                <p class="text-[clamp(1.85rem,2.8vw,2.65rem)] font-bold leading-none tabular-nums text-ink">
                    {summary.runningJobCount}/{summary.activeJobCount}
                </p>
                <p class="mt-1 text-xs text-subtle">Running now and enabled</p>
            </div>
            <div class="absolute inset-x-0 bottom-0 h-0.5 bg-[linear-gradient(90deg,var(--accent),transparent)]"></div>
        </div>

        <div class="relative grid min-h-32 content-between overflow-hidden rounded-sm border border-line bg-panel-strong p-3 shadow-panel motion-safe:animate-[cockpit-enter_380ms_cubic-bezier(0.16,1,0.3,1)_both] motion-safe:[animation-delay:80ms]">
            <div class="flex items-center justify-between gap-2">
                <p class="text-xs font-semibold text-subtle">Failed Items</p>
                <AlertTriangle aria-hidden="true" class="text-subtle" size={17} />
            </div>
            <div>
                <p class="text-[clamp(1.85rem,2.8vw,2.65rem)] font-bold leading-none tabular-nums text-rose-700 dark:text-rose-300">
                    {formatCount(summary.failedItemCount)}
                </p>
                <p class="mt-1 text-xs text-subtle">Across visible runs</p>
            </div>
            <div class="absolute inset-x-0 bottom-0 h-0.5 bg-[linear-gradient(90deg,var(--accent),transparent)]"></div>
        </div>

        <div class="relative grid min-h-32 content-between overflow-hidden rounded-sm border border-line bg-panel-strong p-3 shadow-panel motion-safe:animate-[cockpit-enter_380ms_cubic-bezier(0.16,1,0.3,1)_both] motion-safe:[animation-delay:120ms]">
            <div class="flex items-center justify-between gap-2">
                <p class="text-xs font-semibold text-subtle">Vault Size</p>
                <Archive aria-hidden="true" class="text-subtle" size={17} />
            </div>
            <div>
                <p class="text-[clamp(1.85rem,2.8vw,2.65rem)] font-bold leading-none tabular-nums text-ink">
                    {summary.vaultSizeLabel}
                </p>
                <p class="mt-1 text-xs text-subtle">Estimated from synced items</p>
            </div>
            <div class="absolute inset-x-0 bottom-0 h-0.5 bg-[linear-gradient(90deg,var(--accent),transparent)]"></div>
        </div>
    </div>

    <div class="grid gap-3 xl:grid-cols-12">
        <section class="rounded-sm border border-line bg-panel-strong p-3 shadow-panel motion-safe:animate-[cockpit-enter_380ms_cubic-bezier(0.16,1,0.3,1)_both] xl:col-span-7">
            <div class="mb-2 flex h-7 items-center justify-between gap-3">
                <h2 class="text-sm font-bold text-ink">Run Trend</h2>
                <span class="text-xs text-subtle">Last {Math.min(runs.data.length, 8)}</span>
            </div>
            <RunTrendChart runs={runs.data} />
        </section>

        <section class="overflow-hidden rounded-sm border border-line bg-panel-strong shadow-panel motion-safe:animate-[cockpit-enter_380ms_cubic-bezier(0.16,1,0.3,1)_both] motion-safe:[animation-delay:40ms] xl:col-span-5">
            <div class="flex h-11 items-center justify-between gap-3 border-b border-line px-3">
                <h2 class="text-sm font-bold text-ink">Job Queue</h2>
                <Gauge aria-hidden="true" class="text-subtle" size={16} />
            </div>
            {#if trackedJobs.length > 0}
                <div class="divide-y divide-line-soft">
                    {#each trackedJobs as job (job.id)}
                        <div class="flex min-h-12 items-center justify-between gap-3 px-3 py-2">
                            <div class="min-w-0">
                                <p class="truncate text-sm font-semibold text-ink">
                                    {job.name}
                                </p>
                                <p class="truncate text-xs leading-4 text-subtle">
                                    {job.sourceName} - {job.scheduleLabel}
                                </p>
                            </div>
                            <StatusBadge status={job.status} />
                        </div>
                    {/each}
                </div>
            {:else}
                <div class="px-3 py-8 text-sm text-subtle" role="status">
                    No sync jobs configured yet.
                </div>
            {/if}
        </section>

        <section class="overflow-hidden rounded-sm border border-line bg-panel-strong shadow-panel motion-safe:animate-[cockpit-enter_380ms_cubic-bezier(0.16,1,0.3,1)_both] motion-safe:[animation-delay:80ms] xl:col-span-8">
            <div class="flex h-11 items-center justify-between gap-3 border-b border-line px-3">
                <h2 class="text-sm font-bold text-ink">Recent Runs</h2>
                {#if summary.lastRun}
                    <span class="truncate text-xs text-subtle">
                        Last started {formatDateTime(summary.lastRun.startedAt)}
                    </span>
                {/if}
            </div>

            {#if recentRuns.length > 0}
                <RunSummaryTable
                    runs={recentRuns}
                    selectedRunId={summary.lastRun?.id}
                />
            {:else}
                <div class="px-3 py-8 text-sm text-subtle" role="status">
                    No sync runs have been recorded yet.
                </div>
            {/if}
        </section>

        <section class="overflow-hidden rounded-sm border border-line bg-panel-strong shadow-panel motion-safe:animate-[cockpit-enter_380ms_cubic-bezier(0.16,1,0.3,1)_both] motion-safe:[animation-delay:120ms] xl:col-span-4">
            <div class="flex h-11 items-center justify-between gap-3 border-b border-line px-3">
                <h2 class="text-sm font-bold text-ink">Source Health</h2>
                <Activity aria-hidden="true" class="text-subtle" size={16} />
            </div>
            <div class="divide-y divide-line-soft">
                {#if attentionSources.length > 0}
                    <div class="bg-panel-muted px-3 py-2 text-xs font-semibold text-muted">
                        Attention needed
                    </div>
                    {#each attentionSources as source (source.id)}
                        <div class="flex min-h-12 items-center justify-between gap-3 px-3 py-2">
                            <div class="min-w-0">
                                <p class="truncate text-sm font-semibold text-ink">
                                    {source.name}
                                </p>
                                <p class="truncate text-xs leading-4 text-subtle">
                                    {source.serviceKind} - {formatCount(source.itemCount)} items
                                </p>
                            </div>
                            <StatusBadge status={source.health} />
                        </div>
                    {/each}
                {/if}
                {#each normalSources as source (source.id)}
                    <div class="flex min-h-12 items-center justify-between gap-3 px-3 py-2">
                        <div class="min-w-0">
                            <p class="truncate text-sm font-semibold text-ink">
                                {source.name}
                            </p>
                            <p class="truncate text-xs leading-4 text-subtle">
                                {source.serviceKind} - {formatCount(source.itemCount)} items
                            </p>
                        </div>
                        <StatusBadge status={source.health} />
                    </div>
                {/each}
                {#if attentionSources.length === 0 && normalSources.length === 0}
                    <div class="px-3 py-8 text-sm text-subtle" role="status">
                        No sources configured yet.
                    </div>
                {/if}
            </div>
        </section>
    </div>

    <FallbackNotice error={sources.error ?? jobs.error ?? runs.error} />
</section>
