import * as wasmBindings from "@rivetkit/rivetkit-wasm";
import {
	Registry,
	type RegistryActors,
	type RegistryConfigInput,
	setup as rivetkitSetup,
} from "rivetkit";

// Re-export the rivetkit authoring API (actor, createClient helpers, types,
// etc.) so a Supabase Edge Function's import map can point `rivetkit` at this
// pre-bundled adapter. The user's source still reads `import { actor } from
// "rivetkit"`; the import map redirects it here, so the deploy never pulls
// rivetkit's native dependency closure into the Deno eszip. The local `setup`
// and `serve` below intentionally shadow rivetkit's `setup`, wiring the wasm
// runtime automatically.
export * from "rivetkit";

const DEFAULT_MANAGER_PATH = "/api/rivet";

/** Config passed to `setup` / `serve`. The wasm runtime is wired automatically. */
export type SupabaseSetupConfig<A extends RegistryActors> = Omit<
	RegistryConfigInput<A>,
	"runtime" | "wasm"
>;

/**
 * Wraps rivetkit's `setup` with the Supabase Edge Functions WebAssembly runtime
 * wired in. Returns a typed `Registry`, so you can derive a typed client with
 * `createClient<typeof registry>(...)` and pass the same registry to `serve`.
 *
 * The wasm binary is loaded by `serve` (Deno reads it asynchronously), so this
 * stays synchronous.
 */
export function setup<A extends RegistryActors>(
	config: SupabaseSetupConfig<A>,
): Registry<A> {
	return rivetkitSetup<A>({
		runtime: "wasm",
		wasm: { bindings: wasmBindings },
		noWelcome: true,
		...config,
	} as RegistryConfigInput<A>);
}

export interface ServeOptions {
	/** Path the Rivet manager API is mounted at. Defaults to `/api/rivet`. */
	managerPath?: string;
	/** Handler for requests that fall outside the Rivet manager API path. */
	fetch?: (request: Request) => Response | Promise<Response>;
}

const resolveWasmUrl = (): URL =>
	new URL(
		(
			import.meta as unknown as { resolve(specifier: string): string }
		).resolve("@rivetkit/rivetkit-wasm/rivetkit_wasm_bg.wasm"),
	);

function applyEnv(
	config: RegistryConfigInput<RegistryActors>,
	managerPath: string,
): void {
	if (!config.endpoint) {
		const endpoint = Deno.env.get("RIVET_ENDPOINT");
		if (endpoint) config.endpoint = endpoint;
	}
	if (!config.namespace) {
		const namespace = Deno.env.get("RIVET_NAMESPACE");
		if (namespace) config.namespace = namespace;
	}
	if (!config.token) {
		const token = Deno.env.get("RIVET_TOKEN");
		if (token) config.token = token;
	}
	const poolName = Deno.env.get("RIVET_POOL");
	if (poolName && !config.envoy?.poolName) {
		config.envoy = { ...config.envoy, poolName };
	}
	if (!config.serverless?.basePath) {
		config.serverless = { ...config.serverless, basePath: managerPath };
	}
}

/**
 * Serves Rivet Actors from a Supabase Edge Function (Deno) on the wasm runtime.
 * Accepts either a registry from this package's `setup` or a setup config (which
 * is wired through `setup` for you).
 *
 * The Rivet manager API is mounted at the configured `serverless.basePath`
 * (or `managerPath`, default `/api/rivet`). Requests outside that path are
 * routed to `options.fetch` if provided. The engine endpoint is read from
 * `RIVET_ENDPOINT` (with `RIVET_NAMESPACE`, `RIVET_TOKEN`, `RIVET_POOL` optional)
 * unless set in the config.
 */
export async function serve<A extends RegistryActors>(
	registryOrConfig: Registry<A> | SupabaseSetupConfig<A>,
	options: ServeOptions = {},
): Promise<void> {
	const registry =
		registryOrConfig instanceof Registry
			? registryOrConfig
			: setup(registryOrConfig);

	const wasmModule = await Deno.readFile(resolveWasmUrl());
	const config = registry.config as RegistryConfigInput<RegistryActors>;
	config.wasm = {
		...config.wasm,
		bindings: wasmBindings,
		initInput: wasmModule,
	};

	const managerPath =
		config.serverless?.basePath ??
		options.managerPath ??
		DEFAULT_MANAGER_PATH;
	applyEnv(config, managerPath);

	const managerSegment = `${managerPath}/`;

	Deno.serve((request) => {
		const url = new URL(request.url);

		// Manager request arriving at the configured base path.
		if (
			url.pathname === managerPath ||
			url.pathname.startsWith(managerSegment)
		) {
			return registry.handler(request);
		}

		// Supabase mounts the function under `/functions/v1/<name>`, so the
		// manager API can arrive under a prefix (e.g. `/<name>/api/rivet/...`).
		// Strip any prefix before the manager path before handing off, so the
		// example does not need to configure a Supabase-specific base path.
		const prefixedIndex = url.pathname.indexOf(managerSegment);
		if (prefixedIndex > 0) {
			url.pathname = url.pathname.slice(prefixedIndex);
			return registry.handler(new Request(url, request));
		}

		if (options.fetch) {
			return options.fetch(request);
		}

		return new Response(
			"This is a RivetKit server.\n\nLearn more at https://rivet.dev\n",
		);
	});
}
