<script lang="ts">
  import { AlertCircle } from "lucide-svelte";
  import type { FrontendApiError } from "../lib/types";

  let { error }: { error?: FrontendApiError } = $props();
</script>

{#if error}
  <div class="flex items-start gap-2 rounded-sm border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-950" role="status">
    <AlertCircle aria-hidden="true" class="mt-0.5 shrink-0" size={15} />
    <div class="min-w-0">
      <p class="font-medium">Using mock data until the local API endpoint is available.</p>
      <p class="truncate text-xs text-amber-800">{error.code}{error.status ? ` · HTTP ${error.status}` : ""} · {error.message}</p>
    </div>
  </div>
{/if}
