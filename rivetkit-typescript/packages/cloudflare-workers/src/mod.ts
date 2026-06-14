import * as wasmBindings from "@rivetkit/rivetkit-wasm";
import wasmModule from "@rivetkit/rivetkit-wasm/rivetkit_wasm_bg.wasm";
import {
	Registry,
	type RegistryActors,
	type RegistryConfigInput,
	setup as rivetkitSetup,
} from "rivetkit";
// Installs the fetch-based `globalThis.WebSocket` shim required for the wasm
// runtime's outbound tunnel to the Rivet engine. Imported for its side effect.
import "./websocket";

const DEFAULT_MANAGER_PATH = "/api/rivet";

/** Config passed to `setup` / `createHandler`. The wasm runtime is wired automatically. */
export type CloudflareSetupConfig<A extends RegistryActors> = Omit<
	RegistryConfigInput<A>,
	"runtime" | "wasm"
>;

/**
 * Wraps rivetkit's `setup` with the Cloudflare Workers WebAssembly runtime wired
 * in. Returns a typed `Registry`, so you can derive a typed client with
 * `createClient<typeof registry>(...)` and pass the same registry to
 * `createHandler`.
 */
export function setup<A extends RegistryActors>(
	config: CloudflareSetupConfig<A>,
): Registry<A> {
	return rivetkitSetup<A>({
		runtime: "wasm",
		wasm: { bindings: wasmBindings, initInput: wasmModule },
		noWelcome: true,
		...config,
	} as RegistryConfigInput<A>);
}

export interface CreateHandlerOptions {
	/**
	 * Path the Rivet manager API is mounted at. Defaults to `/api/rivet`.
	 *
	 * `rivet dev` and the engine poll `<managerPath>/metadata`, so changing this
	 * also requires configuring the engine-side serverless runner URL to match.
	 */
	managerPath?: string;
	/**
	 * Handler for requests that fall outside the Rivet manager API path. Accepts
	 * a plain `(request, env, ctx)` handler or a framework `fetch` such as
	 * Hono's `app.fetch`.
	 */
	// biome-ignore lint/suspicious/noExplicitAny: accept any fetch handler shape (e.g. Hono's app.fetch).
	fetch?: (request: Request, ...args: any[]) => Response | Promise<Response>;
}

export interface CloudflareHandler {
	fetch(request: Request, env: unknown, ctx: unknown): Promise<Response>;
}

type EnvRecord = Record<string, string | undefined>;

// Fill connection config from the per-request `env` on first request. On
// Cloudflare the env is not available at module scope, so values that were not
// set explicitly in the config are sourced from `RIVET_*` Worker variables here.
function applyEnv(
	registry: Registry<RegistryActors>,
	env: EnvRecord,
	managerPath: string,
): void {
	const config = registry.config as RegistryConfigInput<RegistryActors>;
	if (!config.endpoint && env.RIVET_ENDPOINT) {
		config.endpoint = env.RIVET_ENDPOINT;
	}
	if (!config.namespace && env.RIVET_NAMESPACE) {
		config.namespace = env.RIVET_NAMESPACE;
	}
	if (!config.token && env.RIVET_TOKEN) {
		config.token = env.RIVET_TOKEN;
	}
	if (env.RIVET_POOL && !config.envoy?.poolName) {
		config.envoy = { ...config.envoy, poolName: env.RIVET_POOL };
	}
	if (!config.serverless?.basePath) {
		config.serverless = { ...config.serverless, basePath: managerPath };
	}
}

/**
 * Creates a Cloudflare Workers handler that hosts Rivet Actors on the wasm
 * runtime. Accepts either a registry from this package's `setup` or a setup
 * config (which is wired through `setup` for you).
 *
 * The Rivet manager API is mounted at `managerPath` (default `/api/rivet`).
 * Requests outside that path are routed to `options.fetch` if provided, letting
 * you mount your own routes alongside Rivet. The engine endpoint is read from
 * `RIVET_ENDPOINT` (with `RIVET_NAMESPACE`, `RIVET_TOKEN`, `RIVET_POOL` optional)
 * unless set in the config.
 */
export function createHandler<A extends RegistryActors>(
	registry: Registry<A>,
	options?: CreateHandlerOptions,
): CloudflareHandler;
export function createHandler<A extends RegistryActors>(
	config: CloudflareSetupConfig<A>,
	options?: CreateHandlerOptions,
): CloudflareHandler;
export function createHandler<A extends RegistryActors>(
	registryOrConfig: Registry<A> | CloudflareSetupConfig<A>,
	options: CreateHandlerOptions = {},
): CloudflareHandler {
	const managerPath = options.managerPath ?? DEFAULT_MANAGER_PATH;
	const registry =
		registryOrConfig instanceof Registry
			? registryOrConfig
			: setup(registryOrConfig);
	let envApplied = false;

	return {
		async fetch(request, env, ctx) {
			if (!envApplied) {
				applyEnv(
					registry as Registry<RegistryActors>,
					(env ?? {}) as EnvRecord,
					managerPath,
				);
				envApplied = true;
			}

			const url = new URL(request.url);
			if (
				url.pathname === managerPath ||
				url.pathname.startsWith(`${managerPath}/`)
			) {
				return registry.handler(request);
			}

			if (options.fetch) {
				return options.fetch(request, env, ctx);
			}

			return new Response(
				"This is a RivetKit server.\n\nLearn more at https://rivet.dev\n",
			);
		},
	};
}
