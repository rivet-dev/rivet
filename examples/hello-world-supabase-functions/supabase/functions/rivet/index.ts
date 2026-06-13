import * as wasmBindings from "@rivetkit/rivetkit-wasm";
import { actor, setup } from "rivetkit";

const resolveModule = (
	import.meta as unknown as { resolve(specifier: string): string }
).resolve;
const wasmModule = await Deno.readFile(
	new URL(resolveModule("@rivetkit/rivetkit-wasm/rivetkit_wasm_bg.wasm")),
);

const counter = actor({
	state: { count: 0 },
	actions: {
		increment: (c, amount = 1) => {
			c.state.count += amount;
			return c.state.count;
		},
		getCount: (c) => c.state.count,
	},
});

const registry = setup({
	runtime: "wasm",
	wasm: { bindings: wasmBindings, initInput: wasmModule },
	use: { counter },
	endpoint: Deno.env.get("RIVET_ENDPOINT"),
});

Deno.serve((request) => registry.handler(request));
