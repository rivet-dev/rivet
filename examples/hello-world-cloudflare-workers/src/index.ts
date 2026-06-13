import * as wasmBindings from "@rivetkit/rivetkit-wasm";
import wasmModule from "@rivetkit/rivetkit-wasm/rivetkit_wasm_bg.wasm";
import { actor, setup } from "rivetkit";
import "./cloudflare-websocket";

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

interface Env {
	RIVET_ENDPOINT: string;
}

let registry: { handler(request: Request): Promise<Response> } | undefined;

function getRegistry(env: Env) {
	registry ??= setup({
		runtime: "wasm",
		wasm: { bindings: wasmBindings, initInput: wasmModule },
		use: { counter },
		endpoint: env.RIVET_ENDPOINT,
	});

	return registry;
}

export default {
	async fetch(request: Request, env: Env): Promise<Response> {
		return await getRegistry(env).handler(request);
	},
};
