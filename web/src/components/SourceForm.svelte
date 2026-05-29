<script lang="ts">
  import { Check, Plus, X } from "lucide-svelte";
  import {
    canSubmitSourceForm,
    sourceServiceOptions,
    sourceTemplateById,
    sourceTemplates,
  } from "../lib/sourceServices";
  import type { SourceFormInput, SourceServiceKind } from "../lib/types";

  type SourceFormMode = "create" | "edit";

  let {
    initialValue,
    mode = "create",
    onSubmit,
    onCancel
  }: {
    initialValue?: SourceFormInput;
    mode?: SourceFormMode;
    onSubmit: (input: SourceFormInput) => Promise<void> | void;
    onCancel?: () => void;
  } = $props();

  let name = $state("");
  let serviceKind = $state<SourceServiceKind>("fs");
  let enabled = $state(true);
  let root = $state("");
  let endpoint = $state("");
  let bucket = $state("");
  let region = $state("");
  let username = $state("");
  let accessKeyId = $state("");
  let secretAccessKey = $state("");
  let token = $state("");
  let privateKey = $state("");
  let dataSourceId = $state("");
  let pageId = $state("");
  let version = $state("");
  let appId = $state("");
  let appSecret = $state("");
  let folderToken = $state("");
  let templateId = $state("");
  let isSaving = $state(false);
  let submitLabel = $derived(mode === "edit" ? "Save Source" : "Add Source");
  let savingLabel = $derived(mode === "edit" ? "Saving" : "Adding");
  let selectedTemplate = $derived(sourceTemplateById(templateId));

  $effect(() => {
    resetFields(initialValue);
  });

  let canSubmit = $derived(
    canSubmitSourceForm({
      name,
      serviceKind,
      root,
      endpoint,
      bucket,
      region,
      username,
      accessKeyId,
      secretAccessKey,
      token,
      dataSourceId,
      pageId,
      appId,
      appSecret,
      folderToken
    })
  );

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
          token: blankToUndefined(token),
          privateKey: blankToUndefined(privateKey),
          dataSourceId: blankToUndefined(dataSourceId),
          pageId: blankToUndefined(pageId),
          version: blankToUndefined(version),
          appId: blankToUndefined(appId),
          appSecret: blankToUndefined(appSecret),
          folderToken: blankToUndefined(folderToken)
        }
      });
      if (mode === "create") {
        resetFields(undefined);
      }
    } finally {
      isSaving = false;
    }
  }

  function blankToUndefined(value: string) {
    const trimmed = value.trim();
    return trimmed.length > 0 ? trimmed : undefined;
  }

  function resetFields(value: SourceFormInput | undefined) {
    name = value?.name ?? "";
    serviceKind = value?.serviceKind ?? "fs";
    enabled = value?.enabled ?? true;
    root = value?.config.root ?? "";
    endpoint = value?.config.endpoint ?? "";
    bucket = value?.config.bucket ?? "";
    region = value?.config.region ?? "";
    username = value?.config.username ?? "";
    accessKeyId = value?.config.accessKeyId ?? "";
    secretAccessKey = value?.config.secretAccessKey ?? "";
    token = value?.config.token ?? "";
    privateKey = value?.config.privateKey ?? "";
    dataSourceId = value?.config.dataSourceId ?? "";
    pageId = value?.config.pageId ?? "";
    version = value?.config.version ?? "";
    appId = value?.config.appId ?? "";
    appSecret = value?.config.appSecret ?? "";
    folderToken = value?.config.folderToken ?? "";
    templateId = "";
  }

  function applyTemplate(nextTemplateId: string) {
    templateId = nextTemplateId;
    const template = sourceTemplateById(nextTemplateId);
    if (!template) {
      return;
    }

    serviceKind = template.serviceKind;
    if (name.trim().length === 0) {
      name = template.label;
    }
    root = template.defaultConfig.root ?? root;
    endpoint = template.defaultConfig.endpoint ?? endpoint;
    bucket = template.defaultConfig.bucket ?? bucket;
    region = template.defaultConfig.region ?? region;
    username = template.defaultConfig.username ?? username;
    accessKeyId = template.defaultConfig.accessKeyId ?? accessKeyId;
    secretAccessKey = template.defaultConfig.secretAccessKey ?? secretAccessKey;
    token = template.defaultConfig.token ?? token;
    privateKey = template.defaultConfig.privateKey ?? privateKey;
    dataSourceId = template.defaultConfig.dataSourceId ?? dataSourceId;
    pageId = template.defaultConfig.pageId ?? pageId;
    version = template.defaultConfig.version ?? version;
    appId = template.defaultConfig.appId ?? appId;
    appSecret = template.defaultConfig.appSecret ?? appSecret;
    folderToken = template.defaultConfig.folderToken ?? folderToken;
  }
</script>

<form class="grid gap-3" onsubmit={(event) => { event.preventDefault(); submit(); }}>
  <label class="grid gap-1">
    <span class="text-xs font-semibold text-muted">Template</span>
    <select
      class="h-9 w-full rounded-sm border border-line bg-panel-strong px-2 text-sm text-ink disabled:bg-panel-muted disabled:text-subtle"
      value={templateId}
      onchange={(event) => applyTemplate(event.currentTarget.value)}
    >
      <option value="">Custom</option>
      {#each sourceTemplates as template (template.id)}
        <option value={template.id}>{template.label}</option>
      {/each}
    </select>
    {#if selectedTemplate}
      <span class="text-xs leading-snug text-subtle">{selectedTemplate.description}</span>
    {/if}
  </label>

  <div class="grid gap-3 md:grid-cols-[1fr_10rem_8rem]">
    <label class="grid gap-1">
      <span class="text-xs font-semibold text-muted">Name</span>
      <input
        class="h-9 w-full rounded-sm border border-line bg-panel-strong px-2 text-sm text-ink placeholder:text-subtle disabled:bg-panel-muted disabled:text-subtle"
        bind:value={name}
        placeholder="Sample archive"
      />
    </label>

    <label class="grid gap-1">
      <span class="text-xs font-semibold text-muted">Source type</span>
      <select class="h-9 w-full rounded-sm border border-line bg-panel-strong px-2 text-sm text-ink disabled:bg-panel-muted disabled:text-subtle" bind:value={serviceKind}>
        {#each sourceServiceOptions as option (option.value)}
          <option value={option.value} disabled={!option.implemented}>
            {option.label}{option.implemented ? "" : " (not available)"}
          </option>
        {/each}
      </select>
    </label>

    <label class="flex items-end gap-2 pb-2 text-sm text-muted">
      <input class="size-4 rounded-sm" type="checkbox" bind:checked={enabled} />
      Enabled
    </label>
  </div>

  {#if serviceKind === "notion"}
    <div class="grid gap-3 md:grid-cols-2">
      <label class="grid gap-1 md:col-span-2">
        <span class="text-xs font-semibold text-muted">Integration token</span>
        <input
          class="h-9 w-full rounded-sm border border-line bg-panel-strong px-2 text-sm text-ink disabled:bg-panel-muted disabled:text-subtle"
          type="password"
          autocomplete="new-password"
          bind:value={token}
          placeholder="secret_..."
        />
      </label>
      <label class="grid gap-1">
        <span class="text-xs font-semibold text-muted">Data source ID</span>
        <input
          class="h-9 w-full rounded-sm border border-line bg-panel-strong px-2 font-mono text-sm text-ink disabled:bg-panel-muted disabled:text-subtle"
          bind:value={dataSourceId}
          placeholder="Notion data source id"
        />
      </label>
      <label class="grid gap-1">
        <span class="text-xs font-semibold text-muted">Page ID</span>
        <input
          class="h-9 w-full rounded-sm border border-line bg-panel-strong px-2 font-mono text-sm text-ink disabled:bg-panel-muted disabled:text-subtle"
          bind:value={pageId}
          placeholder="Optional page id"
        />
      </label>
      <label class="grid gap-1">
        <span class="text-xs font-semibold text-muted">API version</span>
        <input
          class="h-9 w-full rounded-sm border border-line bg-panel-strong px-2 font-mono text-sm text-ink disabled:bg-panel-muted disabled:text-subtle"
          bind:value={version}
          placeholder="2026-03-11"
        />
      </label>
    </div>
  {:else if serviceKind === "feishu"}
    <div class="grid gap-3 md:grid-cols-2">
      <label class="grid gap-1">
        <span class="text-xs font-semibold text-muted">App ID</span>
        <input
          class="h-9 w-full rounded-sm border border-line bg-panel-strong px-2 font-mono text-sm text-ink disabled:bg-panel-muted disabled:text-subtle"
          bind:value={appId}
          placeholder="cli_..."
        />
      </label>
      <label class="grid gap-1">
        <span class="text-xs font-semibold text-muted">App secret</span>
        <input
          class="h-9 w-full rounded-sm border border-line bg-panel-strong px-2 text-sm text-ink disabled:bg-panel-muted disabled:text-subtle"
          type="password"
          autocomplete="new-password"
          bind:value={appSecret}
          placeholder="App secret"
        />
      </label>
      <label class="grid gap-1 md:col-span-2">
        <span class="text-xs font-semibold text-muted">Folder token</span>
        <input
          class="h-9 w-full rounded-sm border border-line bg-panel-strong px-2 font-mono text-sm text-ink disabled:bg-panel-muted disabled:text-subtle"
          bind:value={folderToken}
          placeholder="Feishu Drive folder token"
        />
      </label>
    </div>
  {:else if serviceKind === "fs"}
    <label class="grid gap-1">
      <span class="text-xs font-semibold text-muted">Root path</span>
      <input
        class="h-9 w-full rounded-sm border border-line bg-panel-strong px-2 font-mono text-sm text-ink placeholder:text-subtle disabled:bg-panel-muted disabled:text-subtle"
        bind:value={root}
        placeholder="/Users/alex/Documents/source"
      />
    </label>
  {:else}
    <div class="grid gap-3 md:grid-cols-2">
      <label class="grid gap-1">
        <span class="text-xs font-semibold text-muted">Endpoint</span>
        <input class="h-9 w-full rounded-sm border border-line bg-panel-strong px-2 text-sm text-ink disabled:bg-panel-muted disabled:text-subtle" bind:value={endpoint} />
      </label>

      {#if serviceKind === "s3"}
        <label class="grid gap-1">
          <span class="text-xs font-semibold text-muted">Bucket</span>
          <input class="h-9 w-full rounded-sm border border-line bg-panel-strong px-2 text-sm text-ink disabled:bg-panel-muted disabled:text-subtle" bind:value={bucket} />
        </label>
        <label class="grid gap-1">
          <span class="text-xs font-semibold text-muted">Region</span>
          <input class="h-9 w-full rounded-sm border border-line bg-panel-strong px-2 text-sm text-ink disabled:bg-panel-muted disabled:text-subtle" bind:value={region} />
        </label>
        <label class="grid gap-1">
          <span class="text-xs font-semibold text-muted">Access key</span>
          <input class="h-9 w-full rounded-sm border border-line bg-panel-strong px-2 text-sm text-ink disabled:bg-panel-muted disabled:text-subtle" bind:value={accessKeyId} />
        </label>
        <label class="grid gap-1">
          <span class="text-xs font-semibold text-muted">Secret key</span>
          <input
            class="h-9 w-full rounded-sm border border-line bg-panel-strong px-2 text-sm text-ink disabled:bg-panel-muted disabled:text-subtle"
            type="password"
            autocomplete="new-password"
            bind:value={secretAccessKey}
          />
        </label>
      {:else}
        <label class="grid gap-1">
          <span class="text-xs font-semibold text-muted">Remote root</span>
          <input class="h-9 w-full rounded-sm border border-line bg-panel-strong px-2 text-sm text-ink disabled:bg-panel-muted disabled:text-subtle" bind:value={root} />
        </label>
        <label class="grid gap-1">
          <span class="text-xs font-semibold text-muted">Username</span>
          <input class="h-9 w-full rounded-sm border border-line bg-panel-strong px-2 text-sm text-ink disabled:bg-panel-muted disabled:text-subtle" bind:value={username} />
        </label>
        {#if serviceKind === "sftp"}
          <label class="grid gap-1">
            <span class="text-xs font-semibold text-muted">Private key</span>
            <input
              class="h-9 w-full rounded-sm border border-line bg-panel-strong px-2 text-sm text-ink disabled:bg-panel-muted disabled:text-subtle"
              type="password"
              autocomplete="new-password"
              bind:value={privateKey}
            />
          </label>
        {/if}
        {#if serviceKind === "webdav"}
          <label class="grid gap-1">
            <span class="text-xs font-semibold text-muted">Token</span>
            <input
              class="h-9 w-full rounded-sm border border-line bg-panel-strong px-2 text-sm text-ink disabled:bg-panel-muted disabled:text-subtle"
              type="password"
              autocomplete="new-password"
              bind:value={token}
            />
          </label>
        {/if}
      {/if}
    </div>
  {/if}

  <div class="flex justify-end border-t border-line pt-3">
    {#if onCancel}
      <button
        class="mr-2 inline-flex h-8 min-w-max items-center justify-center gap-1 rounded-sm border border-line bg-panel-strong px-3 text-sm font-semibold text-muted transition hover:bg-panel-muted hover:text-ink active:translate-y-px"
        type="button"
        onclick={onCancel}
      >
        <X aria-hidden="true" size={15} />
        Cancel
      </button>
    {/if}
    <button
      class="inline-flex h-8 min-w-max items-center justify-center gap-1 rounded-sm border border-ink bg-ink px-3 text-sm font-semibold text-panel-strong transition hover:border-accent hover:bg-accent hover:text-white active:translate-y-px disabled:cursor-not-allowed disabled:border-line-soft disabled:bg-panel-muted disabled:text-subtle"
      type="submit"
      disabled={!canSubmit || isSaving}
    >
      {#if mode === "edit"}
        <Check aria-hidden="true" size={15} />
      {:else}
        <Plus aria-hidden="true" size={15} />
      {/if}
      {isSaving ? savingLabel : submitLabel}
    </button>
  </div>
</form>
