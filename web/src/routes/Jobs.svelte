<script lang="ts">
    import { Pencil, Play, Square, TimerReset } from "lucide-svelte";
    import FallbackNotice from "../components/FallbackNotice.svelte";
    import JobForm from "../components/JobForm.svelte";
    import StatusBadge from "../components/StatusBadge.svelte";
    import { formatDateTime } from "../lib/format";
    import type {
        JobFormInput,
        LocalId,
        Loadable,
        SourceDto,
        SyncJobDto,
    } from "../lib/types";

    let {
        jobs,
        sources,
        onCreateJob,
        onUpdateJob,
        onRunJob,
        onStopJob,
    }: {
        jobs: Loadable<SyncJobDto[]>;
        sources: Loadable<SourceDto[]>;
        onCreateJob: (input: JobFormInput) => Promise<void> | void;
        onUpdateJob: (jobId: LocalId, input: JobFormInput) => Promise<void> | void;
        onRunJob: (jobId: LocalId) => Promise<void> | void;
        onStopJob: (jobId: LocalId) => Promise<void> | void;
    } = $props();

    let editingJobId = $state<LocalId | undefined>(undefined);

    function jobToFormInput(job: SyncJobDto): JobFormInput {
        return {
            sourceId: job.sourceId,
            name: job.name,
            enabled: job.enabled,
            schedule: job.schedule,
        };
    }
</script>

<section class="grid gap-3 motion-safe:animate-[cockpit-enter_380ms_cubic-bezier(0.16,1,0.3,1)_both]">
    <div>
        <h1 class="text-[clamp(1.25rem,1.6vw,1.65rem)] font-bold leading-tight text-ink">Jobs</h1>
        <p class="mt-1 max-w-[65ch] text-sm leading-snug text-muted">
            Inspect schedules and start one-off source-to-vault runs.
        </p>
    </div>

    <JobForm sources={sources} onSubmit={onCreateJob} />

    <section class="rounded-sm border border-line bg-panel-strong shadow-panel motion-safe:animate-[cockpit-enter_380ms_cubic-bezier(0.16,1,0.3,1)_both] motion-safe:[animation-delay:40ms]">
        <div
            class="flex items-center justify-between gap-3 border-b border-line px-3 py-2"
        >
            <div class="flex items-center gap-2">
                <TimerReset
                    aria-hidden="true"
                    size={16}
                    class="text-subtle"
                />
                <h2 class="text-sm font-bold text-ink">Source Sync Jobs</h2>
            </div>
            <span class="text-xs text-subtle"
                >{jobs.data.filter((job) => job.enabled).length} enabled</span
            >
        </div>

        {#if jobs.data.length === 0}
            <div class="px-3 py-8 text-sm text-subtle" role="status">
                No source sync jobs configured.
            </div>
        {:else}
            <div class="overflow-x-auto">
                <table
                    class="min-w-full border-collapse text-left text-sm"
                >
                    <thead class="bg-panel-muted text-xs text-subtle">
                        <tr>
                            <th class="whitespace-nowrap px-3 py-2 font-bold">Source</th>
                            <th class="whitespace-nowrap px-3 py-2 font-bold">Schedule</th>
                            <th class="whitespace-nowrap px-3 py-2 font-bold">Status</th>
                            <th class="whitespace-nowrap px-3 py-2 font-bold">Last Run</th>
                            <th class="whitespace-nowrap px-3 py-2 font-bold">Next Run</th>
                            <th class="whitespace-nowrap px-3 py-2 text-right font-bold"
                                >Action</th
                            >
                        </tr>
                    </thead>
                    <tbody>
                        {#each jobs.data as job (job.id)}
                            <tr class="transition-colors hover:bg-panel-muted">
                                <td class="border-t border-line-soft px-3 py-2">
                                    <p class="font-semibold text-ink">
                                        {job.name}
                                    </p>
                                    <p class="font-mono text-xs text-subtle">
                                        {job.sourceName} - {job.id}
                                    </p>
                                </td>
                                <td class="border-t border-line-soft px-3 py-2 text-muted"
                                    >{job.scheduleLabel}</td
                                >
                                <td class="border-t border-line-soft px-3 py-2">
                                    <StatusBadge status={job.status} />
                                </td>
                                <td
                                    class="whitespace-nowrap border-t border-line-soft px-3 py-2 text-muted"
                                    >{formatDateTime(job.lastRunAt)}</td
                                >
                                <td
                                    class="whitespace-nowrap border-t border-line-soft px-3 py-2 text-muted"
                                    >{formatDateTime(job.nextRunAt)}</td
                                >
                                <td class="border-t border-line-soft px-3 py-2 text-right">
                                    <button
                                        class="mr-2 inline-flex h-8 min-w-max items-center justify-center gap-1 rounded-sm border border-line bg-panel-strong px-2 text-sm font-semibold text-muted transition hover:bg-panel-muted hover:text-ink active:translate-y-px disabled:cursor-not-allowed disabled:border-line-soft disabled:bg-panel-muted disabled:text-subtle"
                                        type="button"
                                        disabled={job.status === "running"}
                                        onclick={() =>
                                            (editingJobId =
                                                editingJobId === job.id
                                                    ? undefined
                                                    : job.id)}
                                    >
                                        <Pencil aria-hidden="true" size={14} />
                                        Edit
                                    </button>
                                    {#if job.status === "running"}
                                        <button
                                            class="inline-flex h-8 min-w-max items-center justify-center gap-1 rounded-sm border border-rose-300/70 bg-panel-strong px-2 text-sm font-semibold text-rose-700 transition hover:bg-rose-50 hover:text-rose-800 active:translate-y-px dark:border-rose-500/50 dark:text-rose-200 dark:hover:bg-rose-500/10"
                                            type="button"
                                            onclick={() => onStopJob(job.id)}
                                        >
                                            <Square aria-hidden="true" size={14} />
                                            Stop
                                        </button>
                                    {:else}
                                        <button
                                            class="inline-flex h-8 min-w-max items-center justify-center gap-1 rounded-sm border border-ink bg-ink px-2 text-sm font-semibold text-panel-strong transition hover:border-accent hover:bg-accent hover:text-white active:translate-y-px disabled:cursor-not-allowed disabled:border-line-soft disabled:bg-panel-muted disabled:text-subtle"
                                            type="button"
                                            disabled={!job.enabled}
                                            onclick={() => onRunJob(job.id)}
                                        >
                                            <Play aria-hidden="true" size={14} />
                                            Run Now
                                        </button>
                                    {/if}
                                </td>
                            </tr>
                            {#if editingJobId === job.id}
                                <tr class="bg-panel-muted">
                                    <td class="border-t border-line-soft px-3 py-3" colspan="6">
                                        {#key job.id}
                                            <JobForm
                                                sources={sources}
                                                mode="edit"
                                                initialValue={jobToFormInput(
                                                    job,
                                                )}
                                                onSubmit={async (input) => {
                                                    await onUpdateJob(
                                                        job.id,
                                                        input,
                                                    );
                                                    editingJobId = undefined;
                                                }}
                                                onCancel={() =>
                                                    (editingJobId = undefined)}
                                            />
                                        {/key}
                                    </td>
                                </tr>
                            {/if}
                        {/each}
                    </tbody>
                </table>
            </div>
        {/if}
    </section>

    <FallbackNotice error={jobs.error} />
</section>
