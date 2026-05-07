import wasmModule from "../../../rivetkit-typescript/packages/rivetkit-wasm/pkg/rivetkit_wasm_bg.wasm";
import * as rivetkitWasm from "../../../rivetkit-typescript/packages/rivetkit-wasm/pkg/rivetkit_wasm.js";
import { setup } from "rivetkit";
import { counter } from "./actors/counter/counter.ts";
import { rawHttpActor } from "./actors/http/raw-http.ts";
import { rawWebSocketActor } from "./actors/http/raw-websocket.ts";
import { testCounterSqlite } from "./actors/testing/test-counter-sqlite.ts";

(
	globalThis as typeof globalThis & {
		__rivetkitWasmBindings?: typeof rivetkitWasm;
	}
).__rivetkitWasmBindings = rivetkitWasm;

const registry = setup({
	runtime: "wasm",
	wasm: {
		initInput: wasmModule as WebAssembly.Module,
	},
	test: {
		enabled: true,
		sqliteBackend: "remote",
	},
	noWelcome: true,
	use: {
		counter,
		rawHttpActor,
		rawWebSocketActor,
		testCounterSqlite,
	},
});
const handler = registry.fetchHandler({ path: "/api/rivet" });

function matchesRivetPath(pathname: string) {
	return pathname === "/api/rivet" || pathname.startsWith("/api/rivet/");
}

export default {
	async fetch(request: Request) {
		const url = new URL(request.url);
		if (url.pathname === "/health") {
			return Response.json({ ok: true });
		}
		if (matchesRivetPath(url.pathname)) {
			return handler(request);
		}
		return new Response("not found", { status: 404 });
	},
};
