<script lang="ts">
    import {
        ChevronRight,
        Database,
        FileJson,
        FileText,
        Folder,
        HardDrive,
    } from "lucide-svelte";
    import FallbackNotice from "../components/FallbackNotice.svelte";
    import StatusBadge from "../components/StatusBadge.svelte";
    import { formatBytes, formatDateTime } from "../lib/format";
    import type {
        FileBrowseDto,
        FileEntryDto,
        Loadable,
        LocalId,
        SourceDto,
    } from "../lib/types";

    let {
        sources,
        fileBrowser,
        onBrowse,
    }: {
        sources: Loadable<SourceDto[]>;
        fileBrowser: Loadable<FileBrowseDto | undefined>;
        onBrowse: (sourceId: LocalId, path?: string) => Promise<void> | void;
    } = $props();

    let selectedSourceId = $state<LocalId | undefined>(undefined);
    let initializedSourceId = $state<LocalId | undefined>(undefined);
    let selectedSource = $derived(
        sources.data.find((source) => source.id === selectedSourceId) ?? sources.data[0],
    );
    let currentPath = $derived(fileBrowser.data?.path ?? "");
    let crumbs = $derived(pathCrumbs(currentPath));

    $effect(() => {
        if (!selectedSource) {
            return;
        }

        if (selectedSourceId !== selectedSource.id) {
            selectedSourceId = selectedSource.id;
        }

        if (initializedSourceId !== selectedSource.id) {
            initializedSourceId = selectedSource.id;
            onBrowse(selectedSource.id);
        }
    });

    function selectSource(source: SourceDto) {
        selectedSourceId = source.id;
        initializedSourceId = source.id;
        onBrowse(source.id);
    }

    function browsePath(path = "") {
        if (!selectedSource) {
            return;
        }

        onBrowse(selectedSource.id, path || undefined);
    }

    function openEntry(entry: FileEntryDto) {
        if (entry.kind === "directory") {
            browsePath(entry.path);
        }
    }

    function pathCrumbs(path: string) {
        if (!path) {
            return [];
        }

        const parts = path.split("/").filter(Boolean);
        return parts.map((part, index) => ({
            label: part,
            path: parts.slice(0, index + 1).join("/"),
        }));
    }
</script>

<section class="grid gap-3 motion-safe:animate-[cockpit-enter_380ms_cubic-bezier(0.16,1,0.3,1)_both]">
    <div>
        <h1 class="text-[clamp(1.25rem,1.6vw,1.65rem)] font-bold leading-tight text-ink">Files</h1>
        <p class="mt-1 max-w-[65ch] text-sm leading-snug text-muted">
            Browse synced source data by directory with item status, size, and content metadata.
        </p>
    </div>

    <div class="grid gap-3 xl:grid-cols-[18rem_minmax(0,1fr)]">
        <section class="rounded-sm border border-line bg-panel-strong shadow-panel xl:sticky xl:top-20 xl:h-fit">
            <div class="flex items-center gap-2 border-b border-line px-3 py-2">
                <Database aria-hidden="true" size={16} class="text-subtle" />
                <h2 class="text-sm font-bold text-ink">Sources</h2>
            </div>

            {#if sources.data.length === 0}
                <div class="px-3 py-8 text-sm text-subtle" role="status">
                    No sources are configured yet.
                </div>
            {:else}
                <div class="grid gap-1 p-2">
                    {#each sources.data as source (source.id)}
                        <button
                            type="button"
                            class={`flex min-h-11 w-full items-center gap-2 rounded-sm px-2 py-1.5 text-left transition ${
                                selectedSource?.id === source.id
                                    ? "bg-ink text-panel-strong hover:bg-ink hover:text-panel-strong"
                                    : "text-ink hover:bg-panel-muted"
                            }`}
                            aria-current={selectedSource?.id === source.id ? "true" : undefined}
                            onclick={() => selectSource(source)}
                        >
                            <HardDrive aria-hidden="true" size={15} class="shrink-0" />
                            <span class="min-w-0">
                                <span class="block truncate text-sm font-bold">{source.name}</span>
                                <span class="block truncate text-xs opacity-75">{source.serviceKind}</span>
                            </span>
                        </button>
                    {/each}
                </div>
            {/if}
        </section>

        <section class="rounded-sm border border-line bg-panel-strong shadow-panel">
            <div class="flex flex-wrap items-center justify-between gap-2 border-b border-line px-3 py-2">
                <div class="min-w-0">
                    <h2 class="truncate text-sm font-bold text-ink">
                        {selectedSource?.name ?? "No source selected"}
                    </h2>
                    <div class="mt-1 flex min-w-0 flex-wrap items-center gap-1 text-xs text-subtle">
                        <button
                            type="button"
                            class="rounded-sm px-1 font-mono font-bold text-muted hover:bg-panel-muted hover:text-ink disabled:cursor-default disabled:text-subtle"
                            disabled={!selectedSource}
                            onclick={() => browsePath("")}
                        >
                            /
                        </button>
                        {#each crumbs as crumb (crumb.path)}
                            <ChevronRight aria-hidden="true" size={12} class="text-subtle" />
                            <button
                                type="button"
                                class="max-w-40 truncate rounded-sm px-1 font-mono hover:bg-panel-muted hover:text-ink"
                                onclick={() => browsePath(crumb.path)}
                            >
                                {crumb.label}
                            </button>
                        {/each}
                    </div>
                </div>
                <span class="rounded-sm border border-line bg-panel-muted px-2 py-1 text-xs font-bold text-muted">
                    {fileBrowser.status === "loading"
                        ? "Loading"
                        : `${fileBrowser.data?.entries.length ?? 0} entries`}
                </span>
            </div>

            {#if !selectedSource}
                <div class="px-3 py-10 text-sm text-subtle" role="status">
                    Select or create a source to browse synced files.
                </div>
            {:else if fileBrowser.status === "loading" && !fileBrowser.data}
                <div class="px-3 py-10 text-sm text-subtle" role="status">
                    Loading directory entries...
                </div>
            {:else if !fileBrowser.data || fileBrowser.data.entries.length === 0}
                <div class="px-3 py-10 text-sm text-subtle" role="status">
                    This directory has no synced entries.
                </div>
            {:else}
                <div class="overflow-auto">
                    <table class="min-w-full border-collapse text-left text-sm">
                        <thead class="bg-panel-muted text-xs text-subtle">
                            <tr>
                                <th class="whitespace-nowrap px-3 py-2 font-bold">Name</th>
                                <th class="whitespace-nowrap px-3 py-2 font-bold">Status</th>
                                <th class="whitespace-nowrap px-3 py-2 font-bold">Type</th>
                                <th class="whitespace-nowrap px-3 py-2 text-right font-bold">Size</th>
                                <th class="whitespace-nowrap px-3 py-2 font-bold">Modified</th>
                                <th class="whitespace-nowrap px-3 py-2 font-bold">Hash</th>
                            </tr>
                        </thead>
                        <tbody>
                            {#each fileBrowser.data.entries as entry (entry.path)}
                                <tr class="transition-colors hover:bg-panel-muted">
                                    <td class="max-w-[26rem] border-t border-line-soft px-3 py-2">
                                        {#if entry.kind === "directory"}
                                            <button
                                                type="button"
                                                class="inline-flex max-w-full items-center gap-2 text-left font-mono text-xs font-bold text-ink hover:underline"
                                                onclick={() => openEntry(entry)}
                                            >
                                                <Folder aria-hidden="true" size={15} class="shrink-0 text-accent" />
                                                <span class="truncate">{entry.name}</span>
                                            </button>
                                        {:else}
                                            <span class="inline-flex max-w-full items-center gap-2 font-mono text-xs text-ink">
                                                {#if entry.kind === "virtual_document"}
                                                    <FileJson aria-hidden="true" size={15} class="shrink-0 text-muted" />
                                                {:else}
                                                    <FileText aria-hidden="true" size={15} class="shrink-0 text-muted" />
                                                {/if}
                                                <span class="truncate">{entry.name}</span>
                                            </span>
                                        {/if}
                                    </td>
                                    <td class="border-t border-line-soft px-3 py-2">
                                        {#if entry.item}
                                            <StatusBadge status={entry.item.status} />
                                        {:else}
                                            <span class="text-xs text-subtle">-</span>
                                        {/if}
                                    </td>
                                    <td class="border-t border-line-soft px-3 py-2 text-muted">{entry.kind}</td>
                                    <td class="border-t border-line-soft px-3 py-2 text-right tabular-nums text-muted">
                                        {formatBytes(entry.item?.size)}
                                    </td>
                                    <td class="border-t border-line-soft px-3 py-2 text-muted">
                                        {formatDateTime(entry.item?.modifiedAt)}
                                    </td>
                                    <td class="max-w-[16rem] truncate border-t border-line-soft px-3 py-2 font-mono text-xs text-subtle">
                                        {entry.item?.contentHash ?? "-"}
                                    </td>
                                </tr>
                            {/each}
                        </tbody>
                    </table>
                </div>
            {/if}
        </section>
    </div>

    <FallbackNotice error={sources.error ?? fileBrowser.error} />
</section>
