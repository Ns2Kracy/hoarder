<script lang="ts">
    import { Plus } from "lucide-svelte";
    import type {
        JobFormInput,
        Loadable,
        SourceDto,
    } from "../lib/types";

    let {
        sources,
        onCreate,
    }: {
        sources: Loadable<SourceDto[]>;
        onCreate: (input: JobFormInput) => Promise<void> | void;
    } = $props();

    let sourceId = $state("");
    let name = $state("");
    let enabled = $state(true);
    let scheduleKind = $state<"manual" | "interval">("interval");
    let intervalSeconds = $state(300);
    let isSaving = $state(false);

    $effect(() => {
        if (!sourceId && sources.data[0]) {
            sourceId = sources.data[0].id;
        }
    });

    async function submit() {
        if (!sourceId || !name.trim()) {
            return;
        }

        isSaving = true;
        try {
            await onCreate({
                sourceId,
                name: name.trim(),
                enabled,
                schedule:
                    scheduleKind === "manual"
                        ? { kind: "manual" }
                        : {
                              kind: "interval",
                              intervalSeconds: Math.max(
                                  1,
                                  Math.trunc(intervalSeconds),
                              ),
                          },
            });
            name = "";
        } finally {
            isSaving = false;
        }
    }
</script>

<form
    class="rounded-sm border border-zinc-200 bg-white"
    onsubmit={(event) => {
        event.preventDefault();
        submit();
    }}
>
    <div class="grid gap-2 p-3 lg:grid-cols-[minmax(12rem,1fr)_minmax(12rem,1fr)_9rem_9rem_auto]">
        <label class="space-y-1">
            <span class="text-xs font-medium text-zinc-600">Source</span>
            <select
                class="h-9 w-full rounded-sm border border-zinc-300 bg-white px-2 text-sm"
                bind:value={sourceId}
                disabled={sources.data.length === 0}
            >
                {#each sources.data as source (source.id)}
                    <option value={source.id}>{source.name}</option>
                {/each}
            </select>
        </label>
        <label class="space-y-1">
            <span class="text-xs font-medium text-zinc-600">Job name</span>
            <input
                class="h-9 w-full rounded-sm border border-zinc-300 bg-white px-2 text-sm"
                bind:value={name}
                placeholder="Sync job"
            />
        </label>
        <label class="space-y-1">
            <span class="text-xs font-medium text-zinc-600">Schedule</span>
            <select
                class="h-9 w-full rounded-sm border border-zinc-300 bg-white px-2 text-sm"
                bind:value={scheduleKind}
            >
                <option value="interval">Interval</option>
                <option value="manual">Manual</option>
            </select>
        </label>
        <label class="space-y-1">
            <span class="text-xs font-medium text-zinc-600">Seconds</span>
            <input
                class="h-9 w-full rounded-sm border border-zinc-300 bg-white px-2 text-sm disabled:bg-zinc-100"
                type="number"
                min="1"
                bind:value={intervalSeconds}
                disabled={scheduleKind === "manual"}
            />
        </label>
        <div class="flex items-end gap-2">
            <label class="flex h-9 items-center gap-1 text-sm text-zinc-700">
                <input type="checkbox" bind:checked={enabled} />
                Enabled
            </label>
            <button
                class="inline-flex h-9 items-center gap-1 rounded-sm border border-zinc-900 bg-zinc-900 px-3 text-sm font-medium text-white disabled:cursor-not-allowed disabled:border-zinc-300 disabled:bg-zinc-200 disabled:text-zinc-500"
                type="submit"
                disabled={isSaving || !sourceId || !name.trim()}
            >
                <Plus aria-hidden="true" size={15} />
                Add
            </button>
        </div>
    </div>
</form>
