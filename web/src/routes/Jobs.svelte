<script lang="ts">
    import { Play, TimerReset } from "lucide-svelte";
    import FallbackNotice from "../components/FallbackNotice.svelte";
    import StatusBadge from "../components/StatusBadge.svelte";
    import { formatDateTime } from "../lib/format";
    import type { Loadable, SyncJobDto } from "../lib/types";

    let {
        jobs,
        onRunJob,
    }: {
        jobs: Loadable<SyncJobDto[]>;
        onRunJob: (jobId: string) => Promise<void> | void;
    } = $props();
</script>

<section class="space-y-4">
    <div>
        <h1 class="text-xl font-semibold text-zinc-950">Jobs</h1>
        <p class="mt-1 text-sm text-zinc-600">
            Inspect schedules and start one-off sync runs.
        </p>
    </div>

    <section class="rounded-sm border border-zinc-200 bg-white">
        <div
            class="flex items-center justify-between gap-3 border-b border-zinc-200 px-3 py-2"
        >
            <div class="flex items-center gap-2">
                <TimerReset
                    aria-hidden="true"
                    size={16}
                    class="text-zinc-500"
                />
                <h2 class="text-sm font-semibold text-zinc-900">Sync Jobs</h2>
            </div>
            <span class="text-xs text-zinc-500"
                >{jobs.data.filter((job) => job.enabled).length} enabled</span
            >
        </div>

        {#if jobs.data.length === 0}
            <div class="px-3 py-8 text-sm text-zinc-500" role="status">
                No sync jobs configured.
            </div>
        {:else}
            <div class="overflow-x-auto">
                <table
                    class="min-w-full divide-y divide-zinc-200 text-left text-sm"
                >
                    <thead
                        class="bg-zinc-50 text-xs uppercase tracking-normal text-zinc-500"
                    >
                        <tr>
                            <th class="px-3 py-2 font-semibold">Source</th>
                            <th class="px-3 py-2 font-semibold">Schedule</th>
                            <th class="px-3 py-2 font-semibold">Status</th>
                            <th class="px-3 py-2 font-semibold">Last Run</th>
                            <th class="px-3 py-2 font-semibold">Next Run</th>
                            <th class="px-3 py-2 text-right font-semibold"
                                >Action</th
                            >
                        </tr>
                    </thead>
                    <tbody class="divide-y divide-zinc-100">
                        {#each jobs.data as job (job.id)}
                            <tr class="hover:bg-zinc-50">
                                <td class="px-3 py-2">
                                    <p class="font-medium text-zinc-900">
                                        {job.sourceName}
                                    </p>
                                    <p class="font-mono text-xs text-zinc-500">
                                        {job.id}
                                    </p>
                                </td>
                                <td class="px-3 py-2 text-zinc-700"
                                    >{job.scheduleLabel}</td
                                >
                                <td class="px-3 py-2">
                                    <StatusBadge status={job.status} />
                                </td>
                                <td
                                    class="whitespace-nowrap px-3 py-2 text-zinc-600"
                                    >{formatDateTime(job.lastRunAt)}</td
                                >
                                <td
                                    class="whitespace-nowrap px-3 py-2 text-zinc-600"
                                    >{formatDateTime(job.nextRunAt)}</td
                                >
                                <td class="px-3 py-2 text-right">
                                    <button
                                        class="inline-flex h-8 items-center gap-1 rounded-sm border border-zinc-900 bg-zinc-900 px-2 text-sm font-medium text-white disabled:cursor-not-allowed disabled:border-zinc-300 disabled:bg-zinc-200 disabled:text-zinc-500"
                                        type="button"
                                        disabled={!job.enabled ||
                                            job.status === "running"}
                                        onclick={() => onRunJob(job.id)}
                                    >
                                        <Play aria-hidden="true" size={14} />
                                        Run Now
                                    </button>
                                </td>
                            </tr>
                        {/each}
                    </tbody>
                </table>
            </div>
        {/if}
    </section>

    <FallbackNotice error={jobs.error} />
</section>
