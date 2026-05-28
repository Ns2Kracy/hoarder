<script lang="ts">
    import {
        AlertTriangle,
        Archive,
        Database,
        PlayCircle,
        RefreshCcw,
    } from "lucide-svelte";
    import FallbackNotice from "../components/FallbackNotice.svelte";
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
</script>

<section class="grid gap-3 motion-safe:animate-[cockpit-enter_380ms_cubic-bezier(0.16,1,0.3,1)_both]">
    <div class="flex flex-wrap items-start justify-between gap-3">
        <div>
            <h1 class="text-[clamp(1.25rem,1.6vw,1.65rem)] font-bold leading-tight text-ink">Overview</h1>
            <p class="mt-1 max-w-[65ch] text-sm leading-snug text-muted">
                Local connector sync status and recent activity.
            </p>
        </div>
        <button
            class="inline-flex h-8 min-w-max items-center justify-center gap-1 rounded-sm border border-line bg-panel-strong px-3 text-sm font-semibold text-muted shadow-panel transition hover:bg-panel-muted hover:text-ink active:translate-y-px"
            type="button"
            onclick={onRefresh}
        >
            <RefreshCcw aria-hidden="true" size={15} />
            Refresh
        </button>
    </div>

    <div class="grid gap-3 md:grid-cols-2 xl:grid-cols-[1.25fr_1fr_1fr_0.85fr]">
        <div class="relative min-h-28 overflow-hidden rounded-sm border border-line bg-panel-strong p-3 shadow-panel after:absolute after:inset-x-0 after:bottom-0 after:h-0.5 after:bg-[linear-gradient(90deg,var(--accent),transparent)] motion-safe:animate-[cockpit-enter_380ms_cubic-bezier(0.16,1,0.3,1)_both]">
            <div class="flex items-center justify-between gap-2">
                <p class="text-xs font-semibold text-subtle">Sources</p>
                <Database aria-hidden="true" class="text-subtle" size={17} />
            </div>
            <p class="mt-3 text-[clamp(1.6rem,2.4vw,2.35rem)] font-bold leading-none tabular-nums text-ink">
                {summary.enabledSourceCount}/{summary.sourceCount}
            </p>
            <p class="mt-1 text-xs text-subtle">Enabled connectors</p>
        </div>

        <div class="relative min-h-28 overflow-hidden rounded-sm border border-line bg-panel-strong p-3 shadow-panel after:absolute after:inset-x-0 after:bottom-0 after:h-0.5 after:bg-[linear-gradient(90deg,var(--accent),transparent)] motion-safe:animate-[cockpit-enter_380ms_cubic-bezier(0.16,1,0.3,1)_both] motion-safe:[animation-delay:40ms]">
            <div class="flex items-center justify-between gap-2">
                <p class="text-xs font-semibold text-subtle">Active Jobs</p>
                <PlayCircle
                    aria-hidden="true"
                    class="text-subtle"
                    size={17}
                />
            </div>
            <p class="mt-3 text-[clamp(1.6rem,2.4vw,2.35rem)] font-bold leading-none tabular-nums text-ink">
                {summary.runningJobCount}/{summary.activeJobCount}
            </p>
            <p class="mt-1 text-xs text-subtle">Running now / enabled</p>
        </div>

        <div class="relative min-h-28 overflow-hidden rounded-sm border border-line bg-panel-strong p-3 shadow-panel after:absolute after:inset-x-0 after:bottom-0 after:h-0.5 after:bg-[linear-gradient(90deg,var(--accent),transparent)] motion-safe:animate-[cockpit-enter_380ms_cubic-bezier(0.16,1,0.3,1)_both] motion-safe:[animation-delay:80ms]">
            <div class="flex items-center justify-between gap-2">
                <p class="text-xs font-semibold text-subtle">Failed Items</p>
                <AlertTriangle
                    aria-hidden="true"
                    class="text-subtle"
                    size={17}
                />
            </div>
            <p class="mt-3 text-[clamp(1.6rem,2.4vw,2.35rem)] font-bold leading-none tabular-nums text-rose-700 dark:text-rose-300">
                {formatCount(summary.failedItemCount)}
            </p>
            <p class="mt-1 text-xs text-subtle">Across visible runs</p>
        </div>

        <div class="relative min-h-28 overflow-hidden rounded-sm border border-line bg-panel-strong p-3 shadow-panel after:absolute after:inset-x-0 after:bottom-0 after:h-0.5 after:bg-[linear-gradient(90deg,var(--accent),transparent)] motion-safe:animate-[cockpit-enter_380ms_cubic-bezier(0.16,1,0.3,1)_both] motion-safe:[animation-delay:120ms]">
            <div class="flex items-center justify-between gap-2">
                <p class="text-xs font-semibold text-subtle">Vault Size</p>
                <Archive aria-hidden="true" class="text-subtle" size={17} />
            </div>
            <p class="mt-3 text-[clamp(1.6rem,2.4vw,2.35rem)] font-bold leading-none tabular-nums text-ink">
                {summary.vaultSizeLabel}
            </p>
            <p class="mt-1 text-xs text-subtle">
                Estimated from synced items
            </p>
        </div>
    </div>

    <div class="grid gap-3 xl:grid-cols-[1.25fr_0.75fr]">
        <section class="rounded-sm border border-line bg-panel-strong shadow-panel motion-safe:animate-[cockpit-enter_380ms_cubic-bezier(0.16,1,0.3,1)_both]">
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

        <section class="rounded-sm border border-line bg-panel-strong shadow-panel motion-safe:animate-[cockpit-enter_380ms_cubic-bezier(0.16,1,0.3,1)_both] motion-safe:[animation-delay:40ms]">
            <div class="flex items-center justify-between gap-3 border-b border-line px-3 py-2">
                <h2 class="text-sm font-bold text-ink">
                    Source Health
                </h2>
            </div>
            <div>
                {#each sources.data as source (source.id)}
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
            </div>
        </section>
    </div>

    <FallbackNotice error={sources.error ?? jobs.error ?? runs.error} />
</section>
