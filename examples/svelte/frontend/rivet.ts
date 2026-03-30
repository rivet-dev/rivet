import { createRivetKit } from "@rivetkit/svelte";
import type { registry } from "../src/index.ts";

// Create a single shared RivetKit instance for the app.
// useActor() must be called inside a Svelte component's <script> block.
export const { useActor } = createRivetKit<typeof registry>(
	"http://localhost:6420",
);
