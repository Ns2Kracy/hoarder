<script lang="ts">
  import { Cable, CheckCircle2, FlaskConical } from "lucide-svelte";
  import FallbackNotice from "../components/FallbackNotice.svelte";
  import SourceForm from "../components/SourceForm.svelte";
  import StatusBadge from "../components/StatusBadge.svelte";
  import { formatCount, formatDateTime } from "../lib/format";
  import type { Loadable, SourceDto, SourceFormInput } from "../lib/types";

  let {
    sources,
    onAddSource,
    onTestSource
  }: {
    sources: Loadable<SourceDto[]>;
    onAddSource: (input: SourceFormInput) => Promise<void> | void;
    onTestSource: (sourceId: string) => Promise<void> | void;
  } = $props();

  function configLine(source: SourceDto) {
    if (source.config.root) {
      return source.config.root;
    }

    if (source.config.bucket) {
      return `${source.config.bucket}${source.config.region ? ` · ${source.config.region}` : ""}`;
    }

    return source.config.endpoint ?? "No endpoint configured";
  }
</script>

<section class="space-y-4">
  <div>
    <h1 class="text-xl font-semibold text-zinc-950">Sources</h1>
    <p class="mt-1 text-sm text-zinc-600">Configure OpenDAL-backed sources and validate connectivity.</p>
  </div>

  <section class="rounded-sm border border-zinc-200 bg-white">
    <div class="flex items-center gap-2 border-b border-zinc-200 px-3 py-2">
      <Cable aria-hidden="true" size={16} class="text-zinc-500" />
      <h2 class="text-sm font-semibold text-zinc-900">Add Source</h2>
    </div>
    <div class="p-3">
      <SourceForm onSubmit={onAddSource} />
    </div>
  </section>

  <section class="rounded-sm border border-zinc-200 bg-white">
    <div class="flex items-center justify-between gap-3 border-b border-zinc-200 px-3 py-2">
      <h2 class="text-sm font-semibold text-zinc-900">Configured Sources</h2>
      <span class="text-xs text-zinc-500">{formatCount(sources.data.length)} total</span>
    </div>

    {#if sources.data.length === 0}
      <div class="px-3 py-8 text-sm text-zinc-500" role="status">No sources configured.</div>
    {:else}
      <div class="overflow-x-auto">
        <table class="min-w-full divide-y divide-zinc-200 text-left text-sm">
          <thead class="bg-zinc-50 text-xs uppercase tracking-normal text-zinc-500">
            <tr>
              <th class="px-3 py-2 font-semibold">Name</th>
              <th class="px-3 py-2 font-semibold">Service</th>
              <th class="px-3 py-2 font-semibold">Location</th>
              <th class="px-3 py-2 font-semibold">Health</th>
              <th class="px-3 py-2 text-right font-semibold">Items</th>
              <th class="px-3 py-2 font-semibold">Last Check</th>
              <th class="px-3 py-2 text-right font-semibold">Action</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-zinc-100">
            {#each sources.data as source (source.id)}
              <tr class="align-top hover:bg-zinc-50">
                <td class="px-3 py-2">
                  <div class="font-medium text-zinc-900">{source.name}</div>
                  <div class="mt-0.5 flex items-center gap-1 text-xs text-zinc-500">
                    {#if source.enabled}
                      <CheckCircle2 aria-hidden="true" size={12} />
                      Enabled
                    {:else}
                      Disabled
                    {/if}
                  </div>
                </td>
                <td class="px-3 py-2 font-mono text-xs text-zinc-700">{source.serviceKind}</td>
                <td class="max-w-72 px-3 py-2">
                  <div class="truncate font-mono text-xs text-zinc-700" title={configLine(source)}>{configLine(source)}</div>
                  {#if source.config.access_key_id || source.config.secret_access_key || source.config.token}
                    <div class="mt-1 text-xs text-zinc-500">Secrets redacted</div>
                  {/if}
                </td>
                <td class="px-3 py-2">
                  <StatusBadge status={source.health} />
                  {#if source.lastError}
                    <p class="mt-1 max-w-72 text-xs text-amber-800">{source.lastError}</p>
                  {/if}
                </td>
                <td class="px-3 py-2 text-right tabular-nums text-zinc-700">{formatCount(source.itemCount)}</td>
                <td class="whitespace-nowrap px-3 py-2 text-zinc-600">{formatDateTime(source.lastCheckedAt)}</td>
                <td class="px-3 py-2 text-right">
                  <button
                    class="inline-flex h-8 items-center gap-1 rounded-sm border border-zinc-300 bg-white px-2 text-sm font-medium text-zinc-800 hover:bg-zinc-50"
                    type="button"
                    onclick={() => onTestSource(source.id)}
                  >
                    <FlaskConical aria-hidden="true" size={14} />
                    Test
                  </button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </section>

  <FallbackNotice error={sources.error} />
</section>
