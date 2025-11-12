import { env } from "cloudflare:workers";
import type { Registry, RunConfig } from "rivetkit";
import {
	type ActorHandlerInterface,
	createActorDurableObject,
	type DurableObjectConstructor,
} from "./actor-handler-do";
import { ConfigSchema, type InputConfig } from "./config";
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

interface Handler {
	handler: ExportedHandler<Bindings>;
	ActorHandler: DurableObjectConstructor;
}

export function createHandler<R extends Registry<any>>(
	registry: R,
	inputConfig?: InputConfig,
): Handler {
	// HACK: Cloudflare does not support using `crypto.randomUUID()` before start, so we pass a default value
	//
	// Runner key is not used on Cloudflare
	inputConfig = { ...inputConfig, runnerKey: "" };

	// Parse config
	const config = ConfigSchema.parse(inputConfig);

	// Create config
	const runConfig = {
		...config,
		driver: {
			name: "cloudflare-workers",
			manager: () => new CloudflareActorsManagerDriver(),
			// HACK: We can't build the actor driver until we're inside the Durable Object
			actor: undefined as any,
		},
		getUpgradeWebSocket: () => upgradeWebSocket,
	} satisfies RunConfig;

	// Create Durable Object
	const ActorHandler = createActorDurableObject(registry, runConfig);

	// Create server
	const serverOutputPromise = registry.start(runConfig);

	// Create Cloudflare handler
	const handler = {
		fetch: async (request, cfEnv, ctx) => {
			const serverOutput = await serverOutputPromise;
			const url = new URL(request.url);

			// Inject Rivet env
			const env = Object.assign({ RIVET: serverOutput.client }, cfEnv);

			// Mount Rivet manager API
			if (url.pathname.startsWith(config.managerPath)) {
				const strippedPath = url.pathname.substring(
					config.managerPath.length,
				);
				url.pathname = strippedPath;
				const modifiedRequest = new Request(url.toString(), request);
				return serverOutput.fetch(modifiedRequest, env, ctx);
			}

			if (config.fetch) {
				return config.fetch(request, env, ctx);
			} else {
				return new Response(
					"This is a RivetKit server.\n\nLearn more at https://rivetkit.org\n",
					{ status: 200 },
				);
			}
		},
	} satisfies ExportedHandler<Bindings>;

	return { handler, ActorHandler };
}
