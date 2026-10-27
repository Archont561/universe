<script lang="ts">
import { VERSION } from "@/src/lib/index.ts";
</script>

<svelte:head>
  <title>universe portal v{VERSION}</title>
</svelte:head>

<main class="mx-auto max-w-3xl px-6 py-16">
  <section class="cu-card space-y-4">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-bold tracking-tight">universe</h1>
      <span class="cu-badge font-mono" data-testid="portal-version">
        <span class="i-lucide-package text-emerald-400"></span>
        v{VERSION}
      </span>
    </div>
    <p class="text-sm text-slate-400">
      Portable CEP-15 / CEP-16 static Conda channel and documentation portal.
    </p>
  </section>
</main>
