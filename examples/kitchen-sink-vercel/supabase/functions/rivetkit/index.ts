import { Buffer } from "node:buffer";
import * as rivetkitWasm from "../../../../../rivetkit-typescript/packages/rivetkit-wasm/pkg-deno/rivetkit_wasm.js";
import "../../../../../rivetkit-typescript/packages/rivetkit/dist/tsup/db/mod.js";
import { setup } from "rivetkit";
import { counter } from "../../../src/actors/counter/counter.ts";
import { rawHttpActor } from "../../../src/actors/http/raw-http.ts";
import { rawWebSocketActor } from "../../../src/actors/http/raw-websocket.ts";
import { testCounterSqlite } from "../../../src/actors/testing/test-counter-sqlite.ts";

const wasmBytes = await Deno.readFile(
	new URL("./rivetkit_wasm_bg.wasm", import.meta.url),
);

(
	globalThis as typeof globalThis & {
		Buffer?: typeof Buffer;
		__rivetkitWasmBindings?: typeof rivetkitWasm;
	}
).Buffer ??= Buffer;

(
	globalThis as typeof globalThis & {
		__rivetkitWasmBindings?: typeof rivetkitWasm;
	}
).__rivetkitWasmBindings = rivetkitWasm;

const registry = setup({
	runtime: "wasm",
	wasm: {
		initInput: wasmBytes,
	},
	test: {
		enabled: true,
		sqliteBackend: "remote",
	},
	noWelcome: true,
	startEngine: false,
	use: {
		counter,
		rawHttpActor,
		rawWebSocketActor,
		testCounterSqlite,
	},
});

function matchesRivetPath(pathname: string) {
	return pathname === "/api/rivet" || pathname.includes("/api/rivet/");
}

function normalizeRivetRequest(request: Request) {
	const url = new URL(request.url);
	const marker = "/api/rivet";
	const markerIndex = url.pathname.indexOf(marker);
	if (markerIndex > 0) {
		url.pathname = url.pathname.slice(markerIndex);
		return new Request(url, request);
	}
	return request;
}

async function handler(request: Request) {
	const url = new URL(request.url);
	if (url.pathname === "/health" || url.pathname.endsWith("/health")) {
		return Response.json({ ok: true });
	}
	if (matchesRivetPath(url.pathname)) {
		return registry.handler(normalizeRivetRequest(request));
	}
	return new Response("not found", { status: 404 });
}

const port = Number(Deno.env.get("PORT") ?? "8000");
const hostname = Deno.env.get("HOST") ?? "127.0.0.1";

Deno.serve({ hostname, port }, handler);
