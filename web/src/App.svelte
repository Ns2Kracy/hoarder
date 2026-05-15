<script lang="ts">
    import {
        Activity,
        Database,
        FolderCog,
        Gauge,
        PlaySquare,
        RefreshCcw,
        Settings as SettingsIcon,
    } from "lucide-svelte";
    import { onMount } from "svelte";
    import Jobs from "./routes/Jobs.svelte";
    import Overview from "./routes/Overview.svelte";
    import Runs from "./routes/Runs.svelte";
    import Settings from "./routes/Settings.svelte";
    import Sources from "./routes/Sources.svelte";
    import {
        addSource,
        consoleOrigin,
        createJob,
        isRefreshing,
        jobs,
        loadConsoleData,
        loadRunDetail,
        runs,
        runItems,
        saveSettings,
        selectedRunDetail,
        settings,
        sources,
        summary,
        testSourceConnection,
        triggerJobRun,
    } from "./lib/state";
    import { formatDateTime } from "./lib/format";
    import type { PageId } from "./lib/types";

    const navigation = [
        { id: "overview", label: "Overview", icon: Gauge },
        { id: "sources", label: "Sources", icon: Database },
        { id: "jobs", label: "Jobs", icon: PlaySquare },
        { id: "runs", label: "Runs", icon: Activity },
        { id: "settings", label: "Settings", icon: SettingsIcon },
    ] satisfies { id: PageId; label: string; icon: typeof Gauge }[];

    let activePage = $state<PageId>(pageFromHash());
    let sidebarOpen = $state(false);

    onMount(() => {
        loadConsoleData();
        const handleHashChange = () => {
            activePage = pageFromHash();
        };
        window.addEventListener("hashchange", handleHashChange);
        return () => window.removeEventListener("hashchange", handleHashChange);
    });

    function selectPage(page: PageId) {
        activePage = page;
        window.history.replaceState(null, "", `#${page}`);
    }

    function pageFromHash(): PageId {
        if (typeof window === "undefined") {
            return "overview";
        }

        const hashPage = window.location.hash.replace("#", "");
        return navigation.some((item) => item.id === hashPage)
            ? (hashPage as PageId)
            : "overview";
    }
</script>

<svelte:head>
    <title>Hoarder Console</title>
</svelte:head>

<div class="min-h-screen bg-zinc-100 text-zinc-900">
    <div class="flex min-h-screen">
        <aside
            class="hidden w-60 shrink-0 border-r border-zinc-200 bg-white lg:block"
        >
            <div
                class="flex h-12 items-center gap-2 border-b border-zinc-200 px-3"
            >
                <FolderCog aria-hidden="true" size={19} class="text-blue-700" />
                <div>
                    <p class="text-sm font-semibold leading-4 text-zinc-950">
                        Hoarder
                    </p>
                    <p class="text-xs leading-4 text-zinc-500">
                        Connector Console
                    </p>
                </div>
            </div>
            <nav class="space-y-1 p-2" aria-label="Primary">
                {#each navigation as item (item.id)}
                    {@const Icon = item.icon}
                    <button
                        class={`flex h-9 w-full items-center gap-2 rounded-sm px-2 text-left text-sm font-medium ${
                            activePage === item.id
                                ? "bg-zinc-900 text-white"
                                : "text-zinc-700 hover:bg-zinc-100"
                        }`}
                        type="button"
                        onclick={() => selectPage(item.id)}
                    >
                        <Icon aria-hidden="true" size={16} />
                        {item.label}
                    </button>
                {/each}
            </nav>
        </aside>

        {#if sidebarOpen}
            <button
                class="fixed inset-0 z-20 bg-zinc-950/30 lg:hidden"
                type="button"
                aria-label="Close navigation"
                onclick={() => (sidebarOpen = false)}
            ></button>
            <aside
                class="fixed inset-y-0 left-0 z-30 w-64 border-r border-zinc-200 bg-white lg:hidden"
            >
                <div
                    class="flex h-12 items-center gap-2 border-b border-zinc-200 px-3"
                >
                    <FolderCog
                        aria-hidden="true"
                        size={19}
                        class="text-blue-700"
                    />
                    <div>
                        <p
                            class="text-sm font-semibold leading-4 text-zinc-950"
                        >
                            Hoarder
                        </p>
                        <p class="text-xs leading-4 text-zinc-500">
                            Connector Console
                        </p>
                    </div>
                </div>
                <nav class="space-y-1 p-2" aria-label="Mobile primary">
                    {#each navigation as item (item.id)}
                        {@const Icon = item.icon}
                        <button
                            class={`flex h-9 w-full items-center gap-2 rounded-sm px-2 text-left text-sm font-medium ${
                                activePage === item.id
                                    ? "bg-zinc-900 text-white"
                                    : "text-zinc-700 hover:bg-zinc-100"
                            }`}
                            type="button"
                            onclick={() => {
                                selectPage(item.id);
                                sidebarOpen = false;
                            }}
                        >
                            <Icon aria-hidden="true" size={16} />
                            {item.label}
                        </button>
                    {/each}
                </nav>
            </aside>
        {/if}

        <div class="min-w-0 flex-1">
            <header class="sticky top-0 z-10 border-b border-zinc-200 bg-white">
                <div
                    class="flex h-12 items-center justify-between gap-3 px-3 lg:px-4"
                >
                    <div class="flex min-w-0 items-center gap-2">
                        <button
                            class="inline-flex size-8 items-center justify-center rounded-sm border border-zinc-300 bg-white text-zinc-700 lg:hidden"
                            type="button"
                            aria-label="Open navigation"
                            onclick={() => (sidebarOpen = true)}
                        >
                            <FolderCog aria-hidden="true" size={16} />
                        </button>
                        <div class="min-w-0">
                            <p
                                class="truncate text-sm font-semibold text-zinc-950"
                            >
                                Local API · 127.0.0.1:4761
                            </p>
                            <p class="truncate text-xs text-zinc-500">
                                {$consoleOrigin === "api"
                                    ? "Live API data"
                                    : "Mock data fallback"} · refreshed
                                {formatDateTime(
                                    $sources.updatedAt ??
                                        $jobs.updatedAt ??
                                        $runs.updatedAt,
                                )}
                            </p>
                        </div>
                    </div>

                    <div class="flex items-center gap-2">
                        <span
                            class={`inline-flex h-6 items-center rounded-sm border px-2 text-xs font-medium ${
                                $consoleOrigin === "api"
                                    ? "border-emerald-200 bg-emerald-50 text-emerald-800"
                                    : "border-amber-200 bg-amber-50 text-amber-800"
                            }`}
                        >
                            {$consoleOrigin === "api" ? "API" : "Mock"}
                        </span>
                        <button
                            class="inline-flex size-8 items-center justify-center rounded-sm border border-zinc-300 bg-white text-zinc-700 hover:bg-zinc-50 disabled:opacity-60"
                            type="button"
                            aria-label="Refresh console data"
                            disabled={$isRefreshing}
                            onclick={loadConsoleData}
                        >
                            <RefreshCcw aria-hidden="true" size={15} />
                        </button>
                    </div>
                </div>
            </header>

            <main class="mx-auto max-w-7xl p-3 lg:p-4">
                {#if activePage === "overview"}
                    <Overview
                        summary={$summary}
                        sources={$sources}
                        jobs={$jobs}
                        runs={$runs}
                        onRefresh={loadConsoleData}
                    />
                {:else if activePage === "sources"}
                    <Sources
                        sources={$sources}
                        onAddSource={addSource}
                        onTestSource={testSourceConnection}
                    />
                {:else if activePage === "jobs"}
                    <Jobs
                        jobs={$jobs}
                        sources={$sources}
                        onCreateJob={createJob}
                        onRunJob={triggerJobRun}
                    />
                {:else if activePage === "runs"}
                    <Runs
                        runs={$runs}
                        selectedRunDetail={$selectedRunDetail}
                        runItems={$runItems}
                        onSelectRun={loadRunDetail}
                    />
                {:else if activePage === "settings"}
                    <Settings settings={$settings} onSave={saveSettings} />
                {/if}
            </main>
        </div>
    </div>
</div>
