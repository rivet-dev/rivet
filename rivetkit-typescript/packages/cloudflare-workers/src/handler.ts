// import { env } from "cloudflare:workers";
// import type { Client, Registry, RunConfig } from "rivetkit";
// import {
// 	type ActorHandlerInterface,
// 	createActorDurableObject,
// 	type DurableObjectConstructor,
// } from "./actor-handler-do";
// import { type Config, ConfigSchema, type InputConfig } from "./config";
// import { CloudflareActorsManagerDriver } from "./manager-driver";
// import { upgradeWebSocket } from "./websocket";
//
// /** Cloudflare Workers env */
// export interface Bindings {
// 	ACTOR_KV: KVNamespace;
// 	ACTOR_DO: DurableObjectNamespace<ActorHandlerInterface>;
// }
//
// /**
//  * Stores the env for the current request. Required since some contexts like the inline client driver does not have access to the Hono context.
//  *
//  * Use getCloudflareAmbientEnv unless using CF_AMBIENT_ENV.run.
//  */
// export function getCloudflareAmbientEnv(): Bindings {
// 	return env as unknown as Bindings;
// }
//
// export interface InlineOutput<A extends Registry<any>> {
// 	/** Client to communicate with the actors. */
// 	client: Client<A>;
//
// 	/** Fetch handler to manually route requests to the Rivet manager API. */
// 	fetch: (request: Request, ...args: any) => Response | Promise<Response>;
//
// 	config: Config;
//
// 	ActorHandler: DurableObjectConstructor;
// }
//
// export interface HandlerOutput {
// 	handler: ExportedHandler<Bindings>;
// 	ActorHandler: DurableObjectConstructor;
// }
//
// /**
//  * Creates an inline client for accessing Rivet Actors privately without a public manager API.
//  *
//  * If you want to expose a public manager API, either:
//  *
//  * - Use `createHandler` to expose the Rivet API on `/rivet`
//  * - Forward Rivet API requests to `InlineOutput::fetch`
//  */
// export function createInlineClient<R extends Registry<any>>(
// 	registry: R,
// 	inputConfig?: InputConfig,
// ): InlineOutput<R> {
// 	// HACK: Cloudflare does not support using `crypto.randomUUID()` before start, so we pass a default value
// 	//
// 	// Runner key is not used on Cloudflare
// 	inputConfig = { ...inputConfig, runnerKey: "" };
//
// 	// Parse config
// 	const config = ConfigSchema.parse(inputConfig);
//
// 	// Create config
// 	const runConfig = {
// 		...config,
// 		noWelcome: true,
// 		driver: {
// 			name: "cloudflare-workers",
// 			manager: () => new CloudflareActorsManagerDriver(),
// 			// HACK: We can't build the actor driver until we're inside the Durable Object
// 			actor: undefined as any,
// 		},
// 		getUpgradeWebSocket: () => upgradeWebSocket,
// 	} satisfies RunConfig;
//
// 	// Create Durable Object
// 	const ActorHandler = createActorDurableObject(registry, runConfig);
//
// 	// Create server
// 	const { client, fetch } = registry.start(runConfig);
//
// 	return { client, fetch, config, ActorHandler };
// }
//
// /**
//  * Creates a handler to be exported from a Cloudflare Worker.
//  *
//  * This will automatically expose the Rivet manager API on `/rivet`.
//  *
//  * This includes a `fetch` handler and `ActorHandler` Durable Object.
//  */
// export function createHandler<R extends Registry<any>>(
// 	registry: R,
// 	inputConfig?: InputConfig,
// ): HandlerOutput {
// 	const { client, fetch, config, ActorHandler } = createInlineClient(
// 		registry,
// 		inputConfig,
// 	);
//
// 	// Create Cloudflare handler
// 	const handler = {
// 		fetch: async (request, cfEnv, ctx) => {
// 			const url = new URL(request.url);
//
// 			// Inject Rivet env
// 			const env = Object.assign({ RIVET: client }, cfEnv);
//
// 			// Mount Rivet manager API
// 			if (url.pathname.startsWith(config.managerPath)) {
// 				const strippedPath = url.pathname.substring(
// 					config.managerPath.length,
// 				);
// 				url.pathname = strippedPath;
// 				const modifiedRequest = new Request(url.toString(), request);
// 				return fetch(modifiedRequest, env, ctx);
// 			}
//
// 			if (config.fetch) {
// 				return config.fetch(request, env, ctx);
// 			} else {
// 				return new Response(
// 					"This is a RivetKit server.\n\nLearn more at https://rivetkit.org\n",
// 					{ status: 200 },
// 				);
// 			}
// 		},
// 	} satisfies ExportedHandler<Bindings>;
//
// 	return { handler, ActorHandler };
// }
