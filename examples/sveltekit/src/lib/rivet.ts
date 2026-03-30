import { createRivetContext } from "@rivetkit/svelte";
import type { registry } from "../../server/index.ts";

// Create an app-local typed context. Set up in +layout.svelte, consumed in pages.
export const rivetContext = createRivetContext<typeof registry>("AppRivet");
