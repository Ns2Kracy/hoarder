<script lang="ts">
    import { Save, Settings as SettingsIcon } from "lucide-svelte";
    import FallbackNotice from "../components/FallbackNotice.svelte";
    import type {
        Loadable,
        LogLevel,
        SettingsDto,
        SettingsUpdate,
    } from "../lib/types";

    let {
        settings,
        onSave,
    }: {
        settings: Loadable<SettingsDto>;
        onSave: (nextSettings: SettingsUpdate) => Promise<void> | void;
    } = $props();

    let vaultPath = $state("");
    let databasePath = $state("");
    let listenAddress = $state("");
    let jobConcurrency = $state(1);
    let fileConcurrency = $state(4);
    let logLevel = $state<LogLevel>("info");
    let loadedFrom = $state("");
    let isSaving = $state(false);

    $effect(() => {
        if (settings.updatedAt !== loadedFrom) {
            vaultPath = settings.data.vaultPath;
            databasePath = settings.data.databasePath;
            listenAddress = settings.data.listenAddress;
            jobConcurrency = settings.data.jobConcurrency;
            fileConcurrency = settings.data.fileConcurrency;
            logLevel = settings.data.logLevel;
            loadedFrom = settings.updatedAt ?? "";
        }
    });

    async function submit() {
        isSaving = true;
        try {
            await onSave({
                jobConcurrency,
                fileConcurrency,
                logLevel,
            });
        } finally {
            isSaving = false;
        }
    }
</script>

<section class="grid gap-3 motion-safe:animate-[cockpit-enter_380ms_cubic-bezier(0.16,1,0.3,1)_both]">
    <div>
        <h1 class="text-[clamp(1.25rem,1.6vw,1.65rem)] font-bold leading-tight text-ink">Settings</h1>
        <p class="mt-1 max-w-[65ch] text-sm leading-snug text-muted">
            Local paths, bind address, concurrency, and runtime logging.
        </p>
    </div>

    <form
        class="rounded-sm border border-line bg-panel-strong shadow-panel motion-safe:animate-[cockpit-enter_380ms_cubic-bezier(0.16,1,0.3,1)_both]"
        onsubmit={(event) => {
            event.preventDefault();
            submit();
        }}
    >
        <div class="flex items-center gap-2 border-b border-line px-3 py-2">
            <SettingsIcon aria-hidden="true" size={16} class="text-subtle" />
            <h2 class="text-sm font-bold text-ink">
                Runtime Configuration
            </h2>
        </div>

        <div class="grid gap-3 p-3 lg:grid-cols-2">
            <label class="grid gap-1">
                <span class="text-xs font-semibold text-muted">Vault path</span
                >
                <input
                    class="h-9 w-full rounded-sm border border-line bg-panel-muted px-2 font-mono text-sm text-subtle"
                    bind:value={vaultPath}
                    readonly={settings.data.readOnly.vaultPath}
                    aria-readonly={settings.data.readOnly.vaultPath}
                />
            </label>
            <label class="grid gap-1">
                <span class="text-xs font-semibold text-muted"
                    >Database path</span
                >
                <input
                    class="h-9 w-full rounded-sm border border-line bg-panel-muted px-2 font-mono text-sm text-subtle"
                    bind:value={databasePath}
                    readonly={settings.data.readOnly.databasePath}
                    aria-readonly={settings.data.readOnly.databasePath}
                />
            </label>
            <label class="grid gap-1">
                <span class="text-xs font-semibold text-muted"
                    >Listen address</span
                >
                <input
                    class="h-9 w-full rounded-sm border border-line bg-panel-muted px-2 font-mono text-sm text-subtle"
                    bind:value={listenAddress}
                    readonly={settings.data.readOnly.listenAddress}
                    aria-readonly={settings.data.readOnly.listenAddress}
                />
            </label>
            <label class="grid gap-1">
                <span class="text-xs font-semibold text-muted">Log level</span>
                <select
                    class="h-9 w-full rounded-sm border border-line bg-panel-strong px-2 text-sm text-ink"
                    bind:value={logLevel}
                >
                    <option value="trace">trace</option>
                    <option value="debug">debug</option>
                    <option value="info">info</option>
                    <option value="warn">warn</option>
                    <option value="error">error</option>
                </select>
            </label>
            <label class="grid gap-1">
                <span class="text-xs font-semibold text-muted"
                    >Job concurrency</span
                >
                <input
                    class="h-9 w-full rounded-sm border border-line bg-panel-strong px-2 text-sm text-ink"
                    type="number"
                    min="1"
                    max="16"
                    bind:value={jobConcurrency}
                />
            </label>
            <label class="grid gap-1">
                <span class="text-xs font-semibold text-muted"
                    >File concurrency</span
                >
                <input
                    class="h-9 w-full rounded-sm border border-line bg-panel-strong px-2 text-sm text-ink"
                    type="number"
                    min="1"
                    max="64"
                    bind:value={fileConcurrency}
                />
            </label>
        </div>

        <div class="flex justify-end border-t border-line px-3 py-2">
            <button
                class="inline-flex h-8 min-w-max items-center justify-center gap-1 rounded-sm border border-ink bg-ink px-3 text-sm font-semibold text-panel-strong transition hover:border-accent hover:bg-accent hover:text-white active:translate-y-px disabled:cursor-not-allowed disabled:border-line-soft disabled:bg-panel-muted disabled:text-subtle"
                type="submit"
                disabled={isSaving}
            >
                <Save aria-hidden="true" size={15} />
                {isSaving ? "Saving" : "Save"}
            </button>
        </div>
    </form>

    <FallbackNotice error={settings.error} />
</section>
