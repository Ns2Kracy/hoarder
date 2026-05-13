<script lang="ts">
  import { Save, Settings as SettingsIcon } from "lucide-svelte";
  import FallbackNotice from "../components/FallbackNotice.svelte";
  import type { Loadable, LogLevel, SettingsDto } from "../lib/types";

  let {
    settings,
    onSave
  }: {
    settings: Loadable<SettingsDto>;
    onSave: (nextSettings: SettingsDto) => Promise<void> | void;
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
        vaultPath,
        databasePath,
        listenAddress,
        jobConcurrency,
        fileConcurrency,
        logLevel
      });
    } finally {
      isSaving = false;
    }
  }
</script>

<section class="space-y-4">
  <div>
    <h1 class="text-xl font-semibold text-zinc-950">Settings</h1>
    <p class="mt-1 text-sm text-zinc-600">Local paths, bind address, concurrency, and runtime logging.</p>
  </div>

  <form class="rounded-sm border border-zinc-200 bg-white" onsubmit={(event) => { event.preventDefault(); submit(); }}>
    <div class="flex items-center gap-2 border-b border-zinc-200 px-3 py-2">
      <SettingsIcon aria-hidden="true" size={16} class="text-zinc-500" />
      <h2 class="text-sm font-semibold text-zinc-900">Runtime Configuration</h2>
    </div>

    <div class="grid gap-3 p-3 lg:grid-cols-2">
      <label class="space-y-1">
        <span class="text-xs font-medium text-zinc-600">Vault path</span>
        <input
          class="h-9 w-full rounded-sm border border-zinc-300 bg-white px-2 font-mono text-sm text-zinc-900"
          bind:value={vaultPath}
        />
      </label>
      <label class="space-y-1">
        <span class="text-xs font-medium text-zinc-600">Database path</span>
        <input
          class="h-9 w-full rounded-sm border border-zinc-300 bg-white px-2 font-mono text-sm text-zinc-900"
          bind:value={databasePath}
        />
      </label>
      <label class="space-y-1">
        <span class="text-xs font-medium text-zinc-600">Listen address</span>
        <input class="h-9 w-full rounded-sm border border-zinc-300 bg-white px-2 font-mono text-sm" bind:value={listenAddress} />
      </label>
      <label class="space-y-1">
        <span class="text-xs font-medium text-zinc-600">Log level</span>
        <select class="h-9 w-full rounded-sm border border-zinc-300 bg-white px-2 text-sm" bind:value={logLevel}>
          <option value="trace">trace</option>
          <option value="debug">debug</option>
          <option value="info">info</option>
          <option value="warn">warn</option>
          <option value="error">error</option>
        </select>
      </label>
      <label class="space-y-1">
        <span class="text-xs font-medium text-zinc-600">Job concurrency</span>
        <input
          class="h-9 w-full rounded-sm border border-zinc-300 bg-white px-2 text-sm"
          type="number"
          min="1"
          max="16"
          bind:value={jobConcurrency}
        />
      </label>
      <label class="space-y-1">
        <span class="text-xs font-medium text-zinc-600">File concurrency</span>
        <input
          class="h-9 w-full rounded-sm border border-zinc-300 bg-white px-2 text-sm"
          type="number"
          min="1"
          max="64"
          bind:value={fileConcurrency}
        />
      </label>
    </div>

    <div class="flex justify-end border-t border-zinc-200 px-3 py-2">
      <button
        class="inline-flex h-8 items-center gap-1 rounded-sm border border-zinc-900 bg-zinc-900 px-3 text-sm font-medium text-white disabled:cursor-not-allowed disabled:border-zinc-300 disabled:bg-zinc-200 disabled:text-zinc-500"
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
