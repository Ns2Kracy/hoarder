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

    function needsAttention(source: SourceDto) {
        return (
            source.health === "failed" ||
            source.health === "warning" ||
            source.health === "disabled"
        );
    }
</script>

<section class="grid gap-3 motion-safe:animate-[cockpit-enter_380ms_cubic-bezier(0.16,1,0.3,1)_both]">
    <section class="grid overflow-hidden rounded-sm border border-line bg-panel-strong shadow-panel xl:grid-cols-[minmax(0,1.08fr)_minmax(24rem,0.92fr)]">
        <div class="flex min-h-[16rem] flex-col justify-between gap-6 p-4 lg:p-5">
            <div>
                <h1 class="max-w-[12ch] text-[clamp(2.2rem,5vw,4.65rem)] font-bold leading-[0.95] text-ink">
                    Local sync control
                </h1>
                <p class="mt-3 max-w-[42ch] text-sm leading-snug text-muted">
                    Connect sources, run jobs, and audit recent activity from this local console.
                </p>
            </div>
            <div class="flex flex-wrap items-end justify-between gap-3">
                <div class="flex flex-wrap items-center gap-2">
                    <button
                        class="inline-flex h-9 min-w-max items-center justify-center gap-1 rounded-sm border border-ink bg-ink px-3 text-sm font-semibold text-panel-strong transition hover:border-accent hover:bg-accent hover:text-white active:translate-y-px"
                        type="button"
                        onclick={onRefresh}
                    >
                        <RefreshCcw aria-hidden="true" size={15} />
                        Refresh
                    </button>
                    {#if summary.lastRun}
                        <StatusBadge status={summary.lastRun.status} />
                    {/if}
                </div>
                {#if summary.lastRun}
                    <p class="max-w-full truncate text-right text-xs text-subtle">
                        Last run {formatDateTime(summary.lastRun.startedAt)}
                    </p>
                {:else}
                    <p class="text-right text-xs text-subtle">No runs recorded yet</p>
                {/if}
            </div>
        </div>

        <div class="grid border-t border-line md:grid-cols-2 xl:border-l xl:border-t-0">
            <div class="relative min-h-32 overflow-hidden border-b border-line p-3 md:border-r xl:border-b">
                <div class="flex items-center justify-between gap-2">
                    <p class="text-xs font-semibold text-subtle">Sources</p>
                    <Database aria-hidden="true" class="text-subtle" size={17} />
                </div>
                <p class="mt-4 text-[clamp(1.8rem,3vw,2.8rem)] font-bold leading-none tabular-nums text-ink">
                    {activeSourceLabel}
                </p>
                <p class="mt-1 text-xs text-subtle">Enabled connectors</p>
                <div class="absolute inset-x-0 bottom-0 h-0.5 bg-[linear-gradient(90deg,var(--accent),transparent)]"></div>
            </div>

            <div class="relative min-h-32 overflow-hidden border-b border-line p-3 xl:border-b">
                <div class="flex items-center justify-between gap-2">
                    <p class="text-xs font-semibold text-subtle">Jobs</p>
                    <PlayCircle aria-hidden="true" class="text-subtle" size={17} />
                </div>
                <p class="mt-4 text-[clamp(1.8rem,3vw,2.8rem)] font-bold leading-none tabular-nums text-ink">
                    {summary.runningJobCount}/{summary.activeJobCount}
                </p>
                <p class="mt-1 text-xs text-subtle">Running now and enabled</p>
                <div class="absolute inset-x-0 bottom-0 h-0.5 bg-[linear-gradient(90deg,var(--accent),transparent)]"></div>
            </div>

            <div class="relative min-h-32 overflow-hidden border-b border-line p-3 md:border-b-0 md:border-r">
                <div class="flex items-center justify-between gap-2">
                    <p class="text-xs font-semibold text-subtle">Failed Items</p>
                    <AlertTriangle aria-hidden="true" class="text-subtle" size={17} />
                </div>
                <p class="mt-4 text-[clamp(1.8rem,3vw,2.8rem)] font-bold leading-none tabular-nums text-rose-700 dark:text-rose-300">
                    {formatCount(summary.failedItemCount)}
                </p>
                <p class="mt-1 text-xs text-subtle">Across visible runs</p>
                <div class="absolute inset-x-0 bottom-0 h-0.5 bg-[linear-gradient(90deg,var(--accent),transparent)]"></div>
            </div>

            <div class="relative min-h-32 overflow-hidden p-3">
                <div class="flex items-center justify-between gap-2">
                    <p class="text-xs font-semibold text-subtle">Vault Size</p>
                    <Archive aria-hidden="true" class="text-subtle" size={17} />
                </div>
                <p class="mt-4 text-[clamp(1.8rem,3vw,2.8rem)] font-bold leading-none tabular-nums text-ink">
                    {summary.vaultSizeLabel}
                </p>
                <p class="mt-1 text-xs text-subtle">Estimated from synced items</p>
                <div class="absolute inset-x-0 bottom-0 h-0.5 bg-[linear-gradient(90deg,var(--accent),transparent)]"></div>
            </div>
        </div>
    </section>

    <div class="grid gap-3 xl:grid-cols-[1.12fr_0.88fr]">
        <section class="rounded-sm border border-line bg-panel-strong p-3 shadow-panel motion-safe:animate-[cockpit-enter_380ms_cubic-bezier(0.16,1,0.3,1)_both]">
            <div class="mb-2 flex items-center justify-between gap-3">
                <h2 class="text-sm font-bold text-ink">Run Trend</h2>
                <span class="text-xs text-subtle">Last {Math.min(runs.data.length, 8)}</span>
            </div>
            <RunTrendChart runs={runs.data} />
        </section>

        <section class="rounded-sm border border-line bg-panel-strong shadow-panel motion-safe:animate-[cockpit-enter_380ms_cubic-bezier(0.16,1,0.3,1)_both] motion-safe:[animation-delay:40ms]">
            <div class="flex items-center justify-between gap-3 border-b border-line px-3 py-2">
                <h2 class="text-sm font-bold text-ink">Job Queue</h2>
                <Gauge aria-hidden="true" class="text-subtle" size={16} />
            </div>
            {#if trackedJobs.length > 0}
                <div>
                    {#each trackedJobs as job (job.id)}
                        <div class="flex items-center justify-between gap-3 border-t border-line-soft px-3 py-2 first:border-t-0">
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
    </div>

    <div class="grid gap-3 xl:grid-cols-[1.15fr_0.85fr]">
        <section class="rounded-sm border border-line bg-panel-strong shadow-panel motion-safe:animate-[cockpit-enter_380ms_cubic-bezier(0.16,1,0.3,1)_both] motion-safe:[animation-delay:40ms]">
            <div class="flex items-center justify-between gap-3 border-b border-line px-3 py-2">
                <h2 class="text-sm font-bold text-ink">Recent Runs</h2>
                {#if summary.lastRun}
                    <span class="text-xs text-subtle"
                        >Last started {formatDateTime(
                            summary.lastRun.startedAt,
                        )}</span
                    >
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

        <section class="rounded-sm border border-line bg-panel-strong shadow-panel motion-safe:animate-[cockpit-enter_380ms_cubic-bezier(0.16,1,0.3,1)_both] motion-safe:[animation-delay:80ms]">
            <div class="flex items-center justify-between gap-3 border-b border-line px-3 py-2">
                <h2 class="text-sm font-bold text-ink">
                    Source Health
                </h2>
                <Activity aria-hidden="true" class="text-subtle" size={16} />
            </div>
            <div>
                {#if attentionSources.length > 0}
                    <div class="border-b border-line-soft bg-panel-muted px-3 py-2 text-xs font-semibold text-muted">
                        Attention needed
                    </div>
                    {#each attentionSources as source (source.id)}
                        <div class="flex items-center justify-between gap-3 border-t border-line-soft px-3 py-2 first:border-t-0">
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
                    <div
                        class="flex items-center justify-between gap-3 border-t border-line-soft px-3 py-2 first:border-t-0"
                    >
                        <div class="min-w-0">
                            <p
                                class="truncate text-sm font-semibold"
                            >
                                {source.name}
                            </p>
                            <p class="truncate text-xs leading-4 text-subtle">
                                {source.serviceKind} - {formatCount(
                                    source.itemCount,
                                )} items
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
