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
        updateJob,
        updateSource,
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

<div class="min-h-[100dvh] bg-canvas text-ink [background-image:radial-gradient(circle_at_top_right,var(--accent-soft),transparent_34rem),linear-gradient(90deg,var(--surface-grid)_1px,transparent_1px),linear-gradient(180deg,var(--surface-grid)_1px,transparent_1px)] [background-size:auto,48px_48px,48px_48px]">
    <header class="sticky top-0 z-10 border-b border-line bg-panel-strong/90 shadow-panel backdrop-blur-xl">
        <div class="mx-auto flex h-16 max-w-[1500px] items-center gap-3 px-3 lg:px-5">
            <div class="flex min-w-0 shrink-0 items-center gap-2">
                <FolderCog aria-hidden="true" size={20} class="text-accent" />
                <div class="min-w-0">
                    <p class="truncate text-sm font-bold leading-4">Hoarder</p>
                    <p class="truncate text-xs leading-4 text-subtle">Connector Console</p>
                </div>
            </div>

            <nav class="flex min-w-0 flex-1 items-center gap-1 overflow-x-auto" aria-label="Primary">
                {#each navigation as item (item.id)}
                    {@const Icon = item.icon}
                    <button
                        class={`inline-flex h-9 min-w-max items-center gap-1.5 rounded-sm px-2.5 text-sm font-semibold transition duration-150 hover:-translate-y-px ${
                            activePage === item.id
                                ? "bg-ink text-panel-strong shadow-panel"
                                : "text-muted hover:bg-panel-muted hover:text-ink"
                        }`}
                        type="button"
                        onclick={() => selectPage(item.id)}
                    >
                        <Icon aria-hidden="true" size={15} />
                        {item.label}
                    </button>
                {/each}
            </nav>

            <div class="hidden min-w-0 items-center gap-2 md:flex">
                <div class="min-w-0 text-right">
                    <p class="truncate text-xs font-semibold text-muted">Local API - 127.0.0.1:4761</p>
                    <p class="truncate text-xs leading-4 text-subtle">
                        {$consoleOrigin === "api" ? "Live API data" : "Mock data fallback"} - refreshed
                        {formatDateTime(
                            $sources.updatedAt ??
                                $jobs.updatedAt ??
                                $runs.updatedAt,
                        )}
                    </p>
                </div>
                <span
                    class={`inline-flex h-6 items-center rounded-sm border px-2 text-xs font-bold ${
                        $consoleOrigin === "api"
                            ? "border-emerald-300/60 bg-emerald-500/10 text-emerald-800 dark:text-emerald-200"
                            : "border-amber-300/60 bg-amber-500/10 text-amber-800 dark:text-amber-200"
                    }`}
                >
                    {$consoleOrigin === "api" ? "API" : "Mock"}
                </span>
            </div>

            <button
                class="inline-flex size-9 shrink-0 items-center justify-center rounded-sm border border-line bg-panel-strong text-muted transition hover:bg-panel-muted hover:text-ink active:translate-y-px disabled:cursor-not-allowed disabled:bg-panel-muted disabled:text-subtle"
                type="button"
                aria-label="Refresh console data"
                disabled={$isRefreshing}
                onclick={loadConsoleData}
            >
                <RefreshCcw
                    aria-hidden="true"
                    size={15}
                    class={$isRefreshing ? "animate-refreshing" : ""}
                />
            </button>
        </div>
    </header>

    <main class="mx-auto max-w-[1500px] p-3 lg:p-5">
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
                        onUpdateSource={updateSource}
                    />
                {:else if activePage === "jobs"}
                    <Jobs
                        jobs={$jobs}
                        sources={$sources}
                        onCreateJob={createJob}
                        onUpdateJob={updateJob}
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
