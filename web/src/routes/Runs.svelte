<script lang="ts">
    import { AlertTriangle, ListChecks } from "lucide-svelte";
    import FallbackNotice from "../components/FallbackNotice.svelte";
    import RunSummaryTable from "../components/RunSummaryTable.svelte";
    import StatusBadge from "../components/StatusBadge.svelte";
    import { formatDateTime, formatDuration } from "../lib/format";
    import type {
        Loadable,
        LocalId,
        SyncItemDto,
        SyncRunDto,
    } from "../lib/types";

    let {
        runs,
        selectedRunDetail,
        runItems,
        onSelectRun,
    }: {
        runs: Loadable<SyncRunDto[]>;
        selectedRunDetail: Loadable<SyncRunDto | undefined>;
        runItems: Loadable<SyncItemDto[]>;
        onSelectRun: (runId: LocalId) => Promise<void> | void;
    } = $props();

    let selectedRunId = $state<LocalId | undefined>(undefined);
    let selectedRun = $derived(
        selectedRunDetail.data ??
            runs.data.find((run) => run.id === selectedRunId) ??
            runs.data[0],
    );
    let loadedRunId = $state<LocalId | undefined>(undefined);

    $effect(() => {
        const nextRunId = selectedRunId ?? runs.data[0]?.id;
        if (nextRunId && nextRunId !== loadedRunId) {
            selectedRunId = nextRunId;
            loadedRunId = nextRunId;
            onSelectRun(nextRunId);
        }
    });

    function selectRun(run: SyncRunDto) {
        selectedRunId = run.id;
        loadedRunId = run.id;
        onSelectRun(run.id);
    }
</script>

<section class="grid gap-3 motion-safe:animate-[cockpit-enter_380ms_cubic-bezier(0.16,1,0.3,1)_both]">
    <div>
        <h1 class="text-[clamp(1.25rem,1.6vw,1.65rem)] font-bold leading-tight text-ink">Runs</h1>
        <p class="mt-1 max-w-[65ch] text-sm leading-snug text-muted">
            Review sync outcomes, item counts, and structured errors.
        </p>
    </div>

    <section class="rounded-sm border border-line bg-panel-strong shadow-panel motion-safe:animate-[cockpit-enter_380ms_cubic-bezier(0.16,1,0.3,1)_both]">
        <div class="flex items-center gap-2 border-b border-line px-3 py-2">
            <ListChecks aria-hidden="true" size={16} class="text-subtle" />
            <h2 class="text-sm font-bold text-ink">Run History</h2>
        </div>

        {#if runs.data.length === 0}
            <div class="px-3 py-8 text-sm text-subtle" role="status">
                No sync runs have been recorded yet.
            </div>
        {:else}
            <RunSummaryTable
                runs={runs.data}
                selectedRunId={selectedRun?.id}
                onSelect={selectRun}
            />
        {/if}
    </section>

    {#if selectedRun}
        <section class="rounded-sm border border-line bg-panel-strong shadow-panel motion-safe:animate-[cockpit-enter_380ms_cubic-bezier(0.16,1,0.3,1)_both] motion-safe:[animation-delay:40ms]">
            <div
                class="flex flex-wrap items-center justify-between gap-2 border-b border-line px-3 py-2"
            >
                <div>
                    <h2 class="text-sm font-bold text-ink">
                        {selectedRun.sourceName}
                    </h2>
                    <p class="font-mono text-xs text-subtle">
                        {selectedRun.id}
                    </p>
                </div>
                <StatusBadge status={selectedRun.status} />
            </div>

            <div class="grid gap-3 p-3 md:grid-cols-5">
                <div>
                    <p class="text-xs text-subtle">Processed</p>
                    <p class="text-lg font-bold tabular-nums text-ink">
                        {selectedRun.counts.processed}
                    </p>
                </div>
                <div>
                    <p class="text-xs text-subtle">Synced</p>
                    <p
                        class="text-lg font-bold tabular-nums text-emerald-700 dark:text-emerald-300"
                    >
                        {selectedRun.counts.synced}
                    </p>
                </div>
                <div>
                    <p class="text-xs text-subtle">Skipped</p>
                    <p class="text-lg font-bold tabular-nums text-muted">
                        {selectedRun.counts.skipped}
                    </p>
                </div>
                <div>
                    <p class="text-xs text-subtle">Failed</p>
                    <p class="text-lg font-bold tabular-nums text-rose-700 dark:text-rose-300">
                        {selectedRun.counts.failed}
                    </p>
                </div>
                <div>
                    <p class="text-xs text-subtle">Deleted on source</p>
                    <p
                        class="text-lg font-bold tabular-nums text-orange-700 dark:text-orange-300"
                    >
                        {selectedRun.counts.deleted}
                    </p>
                </div>
            </div>

            <div class="grid gap-3 border-t border-line p-3 md:grid-cols-3">
                <div>
                    <p class="text-xs text-subtle">Started</p>
                    <p class="text-sm text-ink">
                        {formatDateTime(selectedRun.startedAt)}
                    </p>
                </div>
                <div>
                    <p class="text-xs text-subtle">Finished</p>
                    <p class="text-sm text-ink">
                        {formatDateTime(selectedRun.finishedAt)}
                    </p>
                </div>
                <div>
                    <p class="text-xs text-subtle">Duration</p>
                    <p class="text-sm text-ink">
                        {formatDuration(selectedRun.durationMs)}
                    </p>
                </div>
            </div>

            {#if selectedRun.errors.length > 0}
                <div class="border-t border-line p-3">
                    <div class="mb-2 flex items-center gap-2">
                        <AlertTriangle
                            aria-hidden="true"
                            size={16}
                            class="text-rose-600 dark:text-rose-300"
                        />
                        <h3 class="text-sm font-bold text-ink">
                            Errors
                        </h3>
                    </div>
                    <div class="space-y-2">
                        {#each selectedRun.errors as error (error.id)}
                            <div
                                class="rounded-sm border border-rose-300/60 bg-rose-500/10 p-2 text-rose-950 dark:text-rose-100"
                            >
                                <div
                                    class="flex flex-wrap items-center justify-between gap-2"
                                >
                                    <p
                                        class="font-mono text-xs font-bold"
                                    >
                                        {error.code}
                                    </p>
                                    <p class="text-xs opacity-80">
                                        {formatDateTime(error.createdAt)}
                                    </p>
                                </div>
                                <p class="mt-1 text-sm">
                                    {error.message}
                                </p>
                                {#if error.sourcePath}
                                    <p
                                        class="mt-1 truncate font-mono text-xs opacity-90"
                                    >
                                        {error.sourcePath}
                                    </p>
                                {/if}
                                <pre
                                    class="mt-2 max-h-44 overflow-auto rounded-sm border border-rose-300/60 bg-panel-strong p-2 font-mono text-xs text-ink">{JSON.stringify(
                                        error.details ?? {},
                                        null,
                                        2,
                                    )}</pre>
                            </div>
                        {/each}
                    </div>
                </div>
            {/if}

            <div class="border-t border-line p-3">
                <h3 class="mb-2 text-sm font-bold text-ink">Items</h3>
                {#if runItems.data.length === 0}
                    <p class="text-sm text-subtle">No item records for this run.</p>
                {:else}
                    <div class="max-h-72 overflow-auto rounded-sm border border-line">
                        <table class="min-w-full border-collapse text-left text-sm">
                            <thead class="bg-panel-muted text-xs text-subtle">
                                <tr>
                                    <th class="whitespace-nowrap px-2 py-1.5 font-bold">Path</th>
                                    <th class="whitespace-nowrap px-2 py-1.5 font-bold">Type</th>
                                    <th class="whitespace-nowrap px-2 py-1.5 font-bold">Status</th>
                                    <th class="whitespace-nowrap px-2 py-1.5 text-right font-bold">Size</th>
                                </tr>
                            </thead>
                            <tbody>
                                {#each runItems.data as item (item.id)}
                                    <tr class="transition-colors hover:bg-panel-muted">
                                        <td class="max-w-[28rem] truncate border-t border-line-soft px-2 py-1.5 font-mono text-xs text-ink">{item.sourcePath}</td>
                                        <td class="border-t border-line-soft px-2 py-1.5 text-muted">{item.itemType}</td>
                                        <td class="border-t border-line-soft px-2 py-1.5"><StatusBadge status={item.status} /></td>
                                        <td class="border-t border-line-soft px-2 py-1.5 text-right tabular-nums text-muted">{item.size ?? "-"}</td>
                                    </tr>
                                {/each}
                            </tbody>
                        </table>
                    </div>
                {/if}
            </div>
        </section>
    {/if}

    <FallbackNotice error={runs.error ?? selectedRunDetail.error ?? runItems.error} />
</section>
