<script lang="ts">
    import { Check, Plus, X } from "lucide-svelte";
    import type {
        JobFormInput,
        LocalId,
        Loadable,
        SourceDto,
    } from "../lib/types";

    let {
        sources,
        initialValue,
        mode = "create",
        onSubmit,
        onCancel,
    }: {
        sources: Loadable<SourceDto[]>;
        initialValue?: JobFormInput;
        mode?: "create" | "edit";
        onSubmit: (input: JobFormInput) => Promise<void> | void;
        onCancel?: () => void;
    } = $props();

    let sourceId = $state<LocalId | undefined>(undefined);
    let name = $state("");
    let enabled = $state(true);
    let scheduleKind = $state<"manual" | "interval">("interval");
    let intervalSeconds = $state(300);
    let isSaving = $state(false);
    let submitLabel = $derived(mode === "edit" ? "Save" : "Add");
    let savingLabel = $derived(mode === "edit" ? "Saving" : "Adding");

    $effect(() => {
        if (mode === "edit") {
            resetFields(initialValue, sources.data[0]?.id);
        } else if (sourceId === undefined && sources.data[0]) {
            sourceId = sources.data[0].id;
        }
    });

    async function submit() {
        if (sourceId === undefined || !name.trim()) {
            return;
        }

        isSaving = true;
        try {
            await onSubmit({
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
            if (mode === "create") {
                name = "";
            }
        } finally {
            isSaving = false;
        }
    }

    function resetFields(value: JobFormInput | undefined, fallbackSourceId: LocalId | undefined) {
        sourceId = value?.sourceId ?? fallbackSourceId;
        name = value?.name ?? "";
        enabled = value?.enabled ?? true;
        scheduleKind = value?.schedule.kind ?? "interval";
        intervalSeconds =
            value?.schedule.kind === "interval" ? value.schedule.intervalSeconds : 300;
    }
</script>

<form
    class="rounded-sm border border-line bg-panel-strong shadow-panel motion-safe:animate-[cockpit-enter_380ms_cubic-bezier(0.16,1,0.3,1)_both]"
    onsubmit={(event) => {
        event.preventDefault();
        submit();
    }}
>
    <div class="grid gap-2 p-3 lg:grid-cols-[minmax(12rem,1fr)_minmax(12rem,1fr)_9rem_9rem_auto]">
        <label class="grid gap-1">
            <span class="text-xs font-semibold text-muted">Source</span>
            <select
                class="h-9 w-full rounded-sm border border-line bg-panel-strong px-2 text-sm text-ink disabled:bg-panel-muted disabled:text-subtle"
                bind:value={sourceId}
                disabled={sources.data.length === 0}
            >
                {#each sources.data as source (source.id)}
                    <option value={source.id}>{source.name}</option>
                {/each}
            </select>
        </label>
        <label class="grid gap-1">
            <span class="text-xs font-semibold text-muted">Job name</span>
            <input
                class="h-9 w-full rounded-sm border border-line bg-panel-strong px-2 text-sm text-ink placeholder:text-subtle disabled:bg-panel-muted disabled:text-subtle"
                bind:value={name}
                placeholder="Source sync job"
            />
        </label>
        <label class="grid gap-1">
            <span class="text-xs font-semibold text-muted">Schedule</span>
            <select
                class="h-9 w-full rounded-sm border border-line bg-panel-strong px-2 text-sm text-ink disabled:bg-panel-muted disabled:text-subtle"
                bind:value={scheduleKind}
            >
                <option value="interval">Interval</option>
                <option value="manual">Manual</option>
            </select>
        </label>
        <label class="grid gap-1">
            <span class="text-xs font-semibold text-muted">Seconds</span>
            <input
                class="h-9 w-full rounded-sm border border-line bg-panel-strong px-2 text-sm text-ink disabled:bg-panel-muted disabled:text-subtle"
                type="number"
                min="1"
                bind:value={intervalSeconds}
                disabled={scheduleKind === "manual"}
            />
        </label>
        <div class="flex items-end gap-2">
            <label class="flex h-9 items-center gap-1 text-sm text-muted">
                <input type="checkbox" bind:checked={enabled} />
                Enabled
            </label>
            {#if onCancel}
                <button
                    class="inline-flex h-9 min-w-max items-center justify-center gap-1 rounded-sm border border-line bg-panel-strong px-3 text-sm font-semibold text-muted transition hover:bg-panel-muted hover:text-ink active:translate-y-px"
                    type="button"
                    onclick={onCancel}
                >
                    <X aria-hidden="true" size={15} />
                    Cancel
                </button>
            {/if}
            <button
                class="inline-flex h-9 min-w-max items-center justify-center gap-1 rounded-sm border border-ink bg-ink px-3 text-sm font-semibold text-panel-strong transition hover:border-accent hover:bg-accent hover:text-white active:translate-y-px disabled:cursor-not-allowed disabled:border-line-soft disabled:bg-panel-muted disabled:text-subtle"
                type="submit"
                disabled={isSaving || sourceId === undefined || !name.trim()}
            >
                {#if mode === "edit"}
                    <Check aria-hidden="true" size={15} />
                {:else}
                    <Plus aria-hidden="true" size={15} />
                {/if}
                {isSaving ? savingLabel : submitLabel}
            </button>
        </div>
    </div>
</form>
