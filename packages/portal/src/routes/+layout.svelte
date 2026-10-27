<script lang="ts">
import "virtual:uno.css";
import type { Snippet } from "svelte";

let { children }: { children: Snippet } = $props();
</script>

{@render children()}
