<script lang="ts">
  import { Plus } from "lucide-svelte";
  import type { OpenDalServiceKind, SourceFormInput } from "../lib/types";

  let { onSubmit }: { onSubmit: (input: SourceFormInput) => Promise<void> | void } = $props();

  let name = $state("");
  let serviceKind = $state<OpenDalServiceKind>("fs");
  let enabled = $state(true);
  let root = $state("");
  let endpoint = $state("");
  let bucket = $state("");
  let region = $state("");
  let username = $state("");
  let accessKeyId = $state("");
  let secretAccessKey = $state("");
  let token = $state("");
  let isSaving = $state(false);

  let canSubmit = $derived(name.trim().length > 0 && (serviceKind !== "fs" || root.trim().length > 0));

  async function submit() {
    if (!canSubmit || isSaving) {
      return;
    }

    isSaving = true;
    try {
      await onSubmit({
        name: name.trim(),
        serviceKind,
        enabled,
        config: {
          root: blankToUndefined(root),
          endpoint: blankToUndefined(endpoint),
          bucket: blankToUndefined(bucket),
          region: blankToUndefined(region),
          username: blankToUndefined(username),
          accessKeyId: blankToUndefined(accessKeyId),
          secretAccessKey: blankToUndefined(secretAccessKey),
          token: blankToUndefined(token)
        }
      });
      name = "";
      root = "";
      endpoint = "";
      bucket = "";
      region = "";
      username = "";
      accessKeyId = "";
      secretAccessKey = "";
      token = "";
    } finally {
      isSaving = false;
    }
  }

  function blankToUndefined(value: string) {
    const trimmed = value.trim();
    return trimmed.length > 0 ? trimmed : undefined;
  }
</script>

<form class="space-y-3" onsubmit={(event) => { event.preventDefault(); submit(); }}>
  <div class="grid gap-3 md:grid-cols-[1fr_10rem_8rem]">
    <label class="space-y-1">
      <span class="text-xs font-medium text-zinc-600">Name</span>
      <input
        class="h-9 w-full rounded border border-zinc-300 bg-white px-2 text-sm text-zinc-900"
        bind:value={name}
        placeholder="Sample archive"
      />
    </label>

    <label class="space-y-1">
      <span class="text-xs font-medium text-zinc-600">Service</span>
      <select class="h-9 w-full rounded border border-zinc-300 bg-white px-2 text-sm" bind:value={serviceKind}>
        <option value="fs">Filesystem</option>
        <option value="s3">S3</option>
        <option value="webdav">WebDAV</option>
        <option value="sftp">SFTP</option>
      </select>
    </label>

    <label class="flex items-end gap-2 pb-2 text-sm text-zinc-700">
      <input class="size-4 rounded border-zinc-300" type="checkbox" bind:checked={enabled} />
      Enabled
    </label>
  </div>

  {#if serviceKind === "fs"}
    <label class="block space-y-1">
      <span class="text-xs font-medium text-zinc-600">Root path</span>
      <input
        class="h-9 w-full rounded border border-zinc-300 bg-white px-2 font-mono text-sm text-zinc-900"
        bind:value={root}
        placeholder="/Users/alex/Documents/source"
      />
    </label>
  {:else}
    <div class="grid gap-3 md:grid-cols-2">
      <label class="space-y-1">
        <span class="text-xs font-medium text-zinc-600">Endpoint</span>
        <input class="h-9 w-full rounded border border-zinc-300 bg-white px-2 text-sm" bind:value={endpoint} />
      </label>
      <label class="space-y-1">
        <span class="text-xs font-medium text-zinc-600">Bucket / remote root</span>
        <input class="h-9 w-full rounded border border-zinc-300 bg-white px-2 text-sm" bind:value={bucket} />
      </label>
      <label class="space-y-1">
        <span class="text-xs font-medium text-zinc-600">Region</span>
        <input class="h-9 w-full rounded border border-zinc-300 bg-white px-2 text-sm" bind:value={region} />
      </label>
      <label class="space-y-1">
        <span class="text-xs font-medium text-zinc-600">Username</span>
        <input class="h-9 w-full rounded border border-zinc-300 bg-white px-2 text-sm" bind:value={username} />
      </label>
      <label class="space-y-1">
        <span class="text-xs font-medium text-zinc-600">Access key</span>
        <input class="h-9 w-full rounded border border-zinc-300 bg-white px-2 text-sm" bind:value={accessKeyId} />
      </label>
      <label class="space-y-1">
        <span class="text-xs font-medium text-zinc-600">Secret / token</span>
        {#if serviceKind === "s3"}
          <input
            class="h-9 w-full rounded border border-zinc-300 bg-white px-2 text-sm"
            type="password"
            autocomplete="new-password"
            bind:value={secretAccessKey}
          />
        {:else}
          <input
            class="h-9 w-full rounded border border-zinc-300 bg-white px-2 text-sm"
            type="password"
            autocomplete="new-password"
            bind:value={token}
          />
        {/if}
      </label>
    </div>
  {/if}

  <div class="flex justify-end border-t border-zinc-200 pt-3">
    <button
      class="inline-flex h-8 items-center gap-1 rounded border border-zinc-900 bg-zinc-900 px-3 text-sm font-medium text-white disabled:cursor-not-allowed disabled:border-zinc-300 disabled:bg-zinc-200 disabled:text-zinc-500"
      type="submit"
      disabled={!canSubmit || isSaving}
    >
      <Plus aria-hidden="true" size={15} />
      {isSaving ? "Adding" : "Add Source"}
    </button>
  </div>
</form>
