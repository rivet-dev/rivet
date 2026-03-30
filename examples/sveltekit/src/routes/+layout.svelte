<script lang="ts">
	import { rivetContext } from "$lib/rivet.ts";

	let { children } = $props();

	// Provide the RivetKit client to all child routes via Svelte context.
	rivetContext.setup("http://localhost:6420");
</script>

{@render children()}
