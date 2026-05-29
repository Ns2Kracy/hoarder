<script lang="ts">
    import { Cable, CheckCircle2, FlaskConical, Pencil } from "lucide-svelte";
    import FallbackNotice from "../components/FallbackNotice.svelte";
    import SourceForm from "../components/SourceForm.svelte";
    import StatusBadge from "../components/StatusBadge.svelte";
    import { formatCount, formatDateTime } from "../lib/format";
    import type { Loadable, LocalId, SourceDto, SourceFormInput } from "../lib/types";

    let {
        sources,
        onAddSource,
        onTestSource,
        onUpdateSource,
    }: {
        sources: Loadable<SourceDto[]>;
        onAddSource: (input: SourceFormInput) => Promise<void> | void;
        onTestSource: (sourceId: LocalId) => Promise<void> | void;
        onUpdateSource: (
            sourceId: LocalId,
            input: SourceFormInput,
        ) => Promise<void> | void;
    } = $props();

    let editingSourceId = $state<LocalId | undefined>(undefined);

    function configLine(source: SourceDto) {
        if (source.connectorKind === "notion") {
            return stringOption(source.config.data_source_id) ?? stringOption(source.config.page_id) ?? "Notion workspace";
        }

        if (source.connectorKind === "feishu") {
            return stringOption(source.config.folder_token) ?? "Feishu Drive";
        }

        if (source.connectorKind === "plugin") {
            return stringOption(source.config.plugin_id) ?? "Compiled plugin";
        }

        if (source.config.root) {
            return source.config.root;
        }

        if (source.config.bucket) {
            return `${source.config.bucket}${source.config.region ? ` - ${source.config.region}` : ""}`;
        }

        return source.config.endpoint ?? "No endpoint configured";
    }

    function sourceToFormInput(source: SourceDto): SourceFormInput {
        const config = source.config;

        return {
            name: source.name,
            serviceKind: source.serviceKind,
            enabled: source.enabled,
            config: {
                root: stringOption(config.root),
                endpoint: stringOption(config.endpoint),
                bucket: stringOption(config.bucket),
                region: stringOption(config.region),
                username: stringOption(config.username),
                accessKeyId: stringOption(config.access_key_id),
                secretAccessKey: stringOption(config.secret_access_key),
                token: stringOption(config.token),
                privateKey: stringOption(config.private_key),
                dataSourceId: stringOption(config.data_source_id),
                pageId: stringOption(config.page_id),
                version: stringOption(config.version),
                appId: stringOption(config.app_id),
                appSecret: stringOption(config.app_secret),
                folderToken: stringOption(config.folder_token),
            },
        };
    }

    function stringOption(value: unknown) {
        return typeof value === "string" ? value : undefined;
    }

    function canEditSource(source: SourceDto) {
        return source.connectorKind !== "plugin";
    }
</script>

<section class="grid gap-3 motion-safe:animate-[cockpit-enter_380ms_cubic-bezier(0.16,1,0.3,1)_both]">
    <div>
        <h1 class="text-[clamp(1.25rem,1.6vw,1.65rem)] font-bold leading-tight text-ink">Sources</h1>
        <p class="mt-1 max-w-[65ch] text-sm leading-snug text-muted">
            Configure OpenDAL-backed sources and validate connectivity.
        </p>
    </div>

    <div class="grid gap-3 xl:grid-cols-[minmax(20rem,0.72fr)_minmax(0,1.28fr)]">
        <section class="rounded-sm border border-line bg-panel-strong shadow-panel motion-safe:animate-[cockpit-enter_380ms_cubic-bezier(0.16,1,0.3,1)_both] xl:sticky xl:top-20 xl:h-fit">
        <div class="flex items-center gap-2 border-b border-line px-3 py-2">
            <Cable aria-hidden="true" size={16} class="text-subtle" />
            <h2 class="text-sm font-bold text-ink">Add Source</h2>
        </div>
        <div class="p-3">
            <SourceForm onSubmit={onAddSource} />
        </div>
        </section>

        <section class="rounded-sm border border-line bg-panel-strong shadow-panel motion-safe:animate-[cockpit-enter_380ms_cubic-bezier(0.16,1,0.3,1)_both] motion-safe:[animation-delay:40ms]">
        <div
            class="flex items-center justify-between gap-3 border-b border-line px-3 py-2"
        >
            <h2 class="text-sm font-bold text-ink">
                Configured Sources
            </h2>
            <span class="text-xs text-subtle"
                >{formatCount(sources.data.length)} total</span
            >
        </div>

        {#if sources.data.length === 0}
            <div class="px-3 py-8 text-sm text-subtle" role="status">
                No sources configured.
            </div>
        {:else}
            <div class="overflow-x-auto">
                <table
                    class="min-w-full border-collapse text-left text-sm"
                >
                    <thead class="bg-panel-muted text-xs text-subtle">
                        <tr>
                            <th class="whitespace-nowrap px-3 py-2 font-bold">Name</th>
                            <th class="whitespace-nowrap px-3 py-2 font-bold">Service</th>
                            <th class="whitespace-nowrap px-3 py-2 font-bold">Location</th>
                            <th class="whitespace-nowrap px-3 py-2 font-bold">Health</th>
                            <th class="whitespace-nowrap px-3 py-2 text-right font-bold"
                                >Items</th
                            >
                            <th class="whitespace-nowrap px-3 py-2 font-bold">Last Check</th>
                            <th class="whitespace-nowrap px-3 py-2 text-right font-bold"
                                >Action</th
                            >
                        </tr>
                    </thead>
                    <tbody>
                        {#each sources.data as source (source.id)}
                            <tr class="align-top transition-colors hover:bg-panel-muted">
                                <td class="border-t border-line-soft px-3 py-2">
                                    <div class="font-semibold text-ink">
                                        {source.name}
                                    </div>
                                    <div
                                        class="mt-0.5 flex items-center gap-1 text-xs text-subtle"
                                    >
                                        {#if source.enabled}
                                            <CheckCircle2
                                                aria-hidden="true"
                                                size={12}
                                            />
                                            Enabled
                                        {:else}
                                            Disabled
                                        {/if}
                                    </div>
                                </td>
                                <td
                                    class="border-t border-line-soft px-3 py-2 font-mono text-xs text-muted"
                                    >{source.serviceKind}</td
                                >
                                <td class="max-w-72 border-t border-line-soft px-3 py-2">
                                    <div
                                        class="truncate font-mono text-xs text-muted"
                                        title={configLine(source)}
                                    >
                                        {configLine(source)}
                                    </div>
                                    {#if source.config.access_key_id || source.config.secret_access_key || source.config.token || source.config.app_secret}
                                        <div class="mt-1 text-xs text-subtle">
                                            Secrets redacted
                                        </div>
                                    {/if}
                                </td>
                                <td class="border-t border-line-soft px-3 py-2">
                                    <StatusBadge status={source.health} />
                                    {#if source.lastError}
                                        <p
                                            class="mt-1 max-w-72 text-xs text-amber-800 dark:text-amber-200"
                                        >
                                            {source.lastError}
                                        </p>
                                    {/if}
                                </td>
                                <td
                                    class="border-t border-line-soft px-3 py-2 text-right tabular-nums text-muted"
                                    >{formatCount(source.itemCount)}</td
                                >
                                <td
                                    class="whitespace-nowrap border-t border-line-soft px-3 py-2 text-muted"
                                    >{formatDateTime(source.lastCheckedAt)}</td
                                >
                                <td class="border-t border-line-soft px-3 py-2 text-right">
                                    {#if canEditSource(source)}
                                        <button
                                            class="mr-2 inline-flex h-8 min-w-max items-center justify-center gap-1 rounded-sm border border-line bg-panel-strong px-2 text-sm font-semibold text-muted transition hover:bg-panel-muted hover:text-ink active:translate-y-px"
                                            type="button"
                                            onclick={() =>
                                                (editingSourceId =
                                                    editingSourceId === source.id
                                                        ? undefined
                                                        : source.id)}
                                        >
                                            <Pencil aria-hidden="true" size={14} />
                                            Edit
                                        </button>
                                    {/if}
                                    <button
                                        class="inline-flex h-8 min-w-max items-center justify-center gap-1 rounded-sm border border-line bg-panel-strong px-2 text-sm font-semibold text-muted transition hover:bg-panel-muted hover:text-ink active:translate-y-px"
                                        type="button"
                                        onclick={() => onTestSource(source.id)}
                                    >
                                        <FlaskConical
                                            aria-hidden="true"
                                            size={14}
                                        />
                                        Test
                                    </button>
                                </td>
                            </tr>
                            {#if editingSourceId === source.id}
                                <tr class="bg-panel-muted">
                                    <td class="border-t border-line-soft px-3 py-3" colspan="7">
                                        {#key source.id}
                                            <SourceForm
                                                mode="edit"
                                                initialValue={sourceToFormInput(
                                                    source,
                                                )}
                                                onSubmit={async (input) => {
                                                    await onUpdateSource(
                                                        source.id,
                                                        input,
                                                    );
                                                    editingSourceId = undefined;
                                                }}
                                                onCancel={() =>
                                                    (editingSourceId =
                                                        undefined)}
                                            />
                                        {/key}
                                    </td>
                                </tr>
                            {/if}
                        {/each}
                    </tbody>
                </table>
            </div>
        {/if}
        </section>
    </div>

    <FallbackNotice error={sources.error} />
</section>
