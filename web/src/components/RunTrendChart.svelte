<script lang="ts">
  import { LineChart } from "layerchart";
  import type { SyncRunDto } from "../lib/types";

  let { runs }: { runs: SyncRunDto[] } = $props();

  type RunTrendPoint = {
    index: number;
    label: string;
    synced: number;
    failed: number;
  };

  let trendData = $derived<RunTrendPoint[]>(
    runs
      .slice(0, 8)
      .reverse()
      .map((run, index) => ({
        index,
        label: run.sourceName,
        synced: run.counts.synced,
        failed: run.counts.failed
      }))
  );

  let hasTrendData = $derived(trendData.length > 0);
</script>

<div class="h-44 rounded-sm border border-line bg-panel-inset p-2">
  {#if hasTrendData}
    <LineChart
      data={trendData}
      x="index"
      height={150}
      axis={false}
      grid={false}
      rule={false}
      points={trendData.length <= 5}
      series={[
        {
          key: "synced",
          label: "Synced",
          value: "synced",
          color: "var(--accent)"
        },
        {
          key: "failed",
          label: "Failed",
          value: "failed",
          color: "#e11d48"
        }
      ]}
      props={{
        spline: { strokeWidth: 2.5 },
        points: { r: 3 },
        tooltip: {
          root: {
            classes: {
              root: "rounded-sm border border-line bg-panel-strong px-2 py-1 text-xs text-ink shadow-panel"
            }
          }
        }
      }}
    />
  {:else}
    <div class="flex h-full items-center justify-center text-sm text-subtle" role="status">
      No run trend yet.
    </div>
  {/if}
</div>
