import { env } from "cloudflare:workers";
import type { Client, Registry } from "rivetkit";
import { createClientWithDriver } from "rivetkit";
import { buildManagerRouter } from "rivetkit/driver-helpers";
import {
	type ActorHandlerInterface,
	createActorDurableObject,
	type DurableObjectConstructor,
} from "./actor-handler-do";
import { type Config, ConfigSchema, type InputConfig } from "./config";
import { CloudflareActorsManagerDriver } from "./manager-driver";
import { upgradeWebSocket } from "./websocket";

/** Cloudflare Workers env */
export interface Bindings {
	ACTOR_KV: KVNamespace;
	ACTOR_DO: DurableObjectNamespace<ActorHandlerInterface>;
}

/**
 * Stores the env for the current request. Required since some contexts like the inline client driver does not have access to the Hono context.
 *
 * Use getCloudflareAmbientEnv unless using CF_AMBIENT_ENV.run.
 */
export function getCloudflareAmbientEnv(): Bindings {
	return env as unknown as Bindings;
}

export interface InlineOutput<A extends Registry<any>> {
	/** Client to communicate with the actors. */
	client: Client<A>;

	/** Fetch handler to manually route requests to the Rivet manager API. */
	fetch: (request: Request, ...args: any) => Response | Promise<Response>;

	config: Config;

	ActorHandler: DurableObjectConstructor;
}

export interface HandlerOutput {
	handler: ExportedHandler<Bindings>;
	ActorHandler: DurableObjectConstructor;
}

/**
 * Creates an inline client for accessing Rivet Actors privately without a public manager API.
 *
 * If you want to expose a public manager API, either:
 *
 * - Use `createHandler` to expose the Rivet API on `/api/rivet`
 * - Forward Rivet API requests to `InlineOutput::fetch`
 */
export function createInlineClient<R extends Registry<any>>(
	registry: R,
	inputConfig?: InputConfig,
): InlineOutput<R> {
	// HACK: Cloudflare does not support using `crypto.randomUUID()` before start, so we pass a default value
	//
	// Runner key is not used on Cloudflare
	inputConfig = { ...inputConfig, runnerKey: "" };

	// Parse config
	const config = ConfigSchema.parse(inputConfig);

	// Create Durable Object
	const ActorHandler = createActorDurableObject(
		registry,
		() => upgradeWebSocket,
	);

	// Configure registry for cloudflare-workers
	registry.config.noWelcome = true;
	// Disable inspector since it's not supported on Cloudflare Workers
	registry.config.inspector = {
		enabled: false,
		token: () => "",
	};
	// Set manager base path to "/" since the cloudflare handler strips the /api/rivet prefix
	registry.config.managerBasePath = "/";
	const parsedConfig = registry.parseConfig();

	// Create manager driver
	const managerDriver = new CloudflareActorsManagerDriver();

	// Build the manager router (has actor management endpoints like /actors)
	const { router } = buildManagerRouter(
		parsedConfig,
		managerDriver,
		() => upgradeWebSocket,
	);

	// Create client using the manager driver
	// Avoid excessive generic expansion in DTS generation.
	const client = (createClientWithDriver as any)(
		managerDriver,
	) as Client<R>;

	return { client, fetch: router.fetch.bind(router), config, ActorHandler };
}

/**
 * Creates a handler to be exported from a Cloudflare Worker.
 *
 * This will automatically expose the Rivet manager API on `/api/rivet`.
 *
 * This includes a `fetch` handler and `ActorHandler` Durable Object.
 */
export function createHandler(
	registry: Registry<any>,
	inputConfig?: InputConfig,
): HandlerOutput {
	const inline = (createInlineClient as any)(registry, inputConfig);
	const client = inline.client as any;
	const fetch = inline.fetch as (
		request: Request,
		...args: any
	) => Response | Promise<Response>;
	const config = inline.config as Config;
	const ActorHandler = inline.ActorHandler as DurableObjectConstructor;

	// Create Cloudflare handler
	const handler = {
		fetch: async (request, cfEnv, ctx) => {
			const url = new URL(request.url);

			// Inject Rivet env
			const env = Object.assign({ RIVET: client }, cfEnv);

			// Mount Rivet manager API
			if (url.pathname.startsWith(config.managerPath)) {
				const strippedPath = url.pathname.substring(
					config.managerPath.length,
				);
				url.pathname = strippedPath;
				const modifiedRequest = new Request(url.toString(), request);
				return fetch(modifiedRequest, env, ctx);
			}

			if (config.fetch) {
				return config.fetch(request, env, ctx);
			} else {
				return new Response(
					"This is a RivetKit server.\n\nLearn more at https://rivet.dev\n",
					{ status: 200 },
				);
			}
		},
	} satisfies ExportedHandler<Bindings>;

	return { handler, ActorHandler };
}
