import type { Context as HonoContext } from "hono";
import type { Encoding, UniversalWebSocket } from "rivetkit";
import {
	type ActorOutput,
	type CreateInput,
	type GetForIdInput,
	type GetOrCreateWithKeyInput,
	type GetWithKeyInput,
	generateRandomString,
	type ManagerDisplayInformation,
	type ManagerDriver,
	WS_PROTOCOL_ACTOR,
	WS_PROTOCOL_CONN_PARAMS,
	WS_PROTOCOL_ENCODING,
	WS_PROTOCOL_STANDARD,
	WS_PROTOCOL_TARGET,
} from "rivetkit/driver-helpers";
import { ActorAlreadyExists, InternalError } from "rivetkit/errors";
import { getCloudflareAmbientEnv } from "./handler";
import { logger } from "./log";
import type { Bindings } from "./mod";
import { serializeKey, serializeNameAndKey } from "./util";

// Actor metadata structure
interface ActorData {
	name: string;
	key: string[];
}

const KEYS = {
	ACTOR: {
		// Combined key for actor metadata (name and key)
		metadata: (actorId: string) => `actor:${actorId}:metadata`,

		// Key index function for actor lookup
		keyIndex: (name: string, key: string[] = []) => {
			// Use serializeKey for consistent handling of all keys
			return `actor_key:${serializeKey(key)}`;
		},
	},
};

const STANDARD_WEBSOCKET_HEADERS = [
	"connection",
	"upgrade",
	"sec-websocket-key",
	"sec-websocket-version",
	"sec-websocket-protocol",
	"sec-websocket-extensions",
];

export class CloudflareActorsManagerDriver implements ManagerDriver {
	async sendRequest(
		actorId: string,
		actorRequest: Request,
	): Promise<Response> {
		const env = getCloudflareAmbientEnv();

		logger().debug({
			msg: "sending request to durable object",
			actorId,
			method: actorRequest.method,
			url: actorRequest.url,
		});

		const id = env.ACTOR_DO.idFromString(actorId);
		const stub = env.ACTOR_DO.get(id);

		return await stub.fetch(actorRequest);
	}

	async openWebSocket(
		path: string,
		actorId: string,
		encoding: Encoding,
		params: unknown,
	): Promise<UniversalWebSocket> {
		const env = getCloudflareAmbientEnv();

		logger().debug({
			msg: "opening websocket to durable object",
			actorId,
			path,
		});

		// Make a fetch request to the Durable Object with WebSocket upgrade
		const id = env.ACTOR_DO.idFromString(actorId);
		const stub = env.ACTOR_DO.get(id);

		const protocols: string[] = [];
		protocols.push(WS_PROTOCOL_STANDARD);
		protocols.push(`${WS_PROTOCOL_TARGET}actor`);
		protocols.push(`${WS_PROTOCOL_ACTOR}${actorId}`);
		protocols.push(`${WS_PROTOCOL_ENCODING}${encoding}`);
		if (params) {
			protocols.push(
				`${WS_PROTOCOL_CONN_PARAMS}${encodeURIComponent(JSON.stringify(params))}`,
			);
		}

		const headers: Record<string, string> = {
			Upgrade: "websocket",
			Connection: "Upgrade",
			"sec-websocket-protocol": protocols.join(", "),
		};

		// Use the path parameter to determine the URL
		const normalizedPath = path.startsWith("/") ? path : `/${path}`;
		const url = `http://actor${normalizedPath}`;

		logger().debug({ msg: "rewriting websocket url", from: path, to: url });

		const response = await stub.fetch(url, {
			headers,
		});
		const webSocket = response.webSocket;

		if (!webSocket) {
			throw new InternalError(
				`missing websocket connection in response from DO\n\nStatus: ${response.status}\nResponse: ${await response.text()}`,
			);
		}

		logger().debug({
			msg: "durable object websocket connection open",
			actorId,
		});

		webSocket.accept();

		// TODO: Is this still needed?
		// HACK: Cloudflare does not call onopen automatically, so we need
		// to call this on the next tick
		setTimeout(() => {
			const event = new Event("open");
			(webSocket as any).onopen?.(event);
			(webSocket as any).dispatchEvent(event);
		}, 0);

		return webSocket as unknown as UniversalWebSocket;
	}

	async proxyRequest(
		c: HonoContext<{ Bindings: Bindings }>,
		actorRequest: Request,
		actorId: string,
	): Promise<Response> {
		logger().debug({
			msg: "forwarding request to durable object",
			actorId,
			method: actorRequest.method,
			url: actorRequest.url,
		});

		const id = c.env.ACTOR_DO.idFromString(actorId);
		const stub = c.env.ACTOR_DO.get(id);

		return await stub.fetch(actorRequest);
	}

	async proxyWebSocket(
		c: HonoContext<{ Bindings: Bindings }>,
		path: string,
		actorId: string,
		encoding: Encoding,
		params: unknown,
	): Promise<Response> {
		logger().debug({
			msg: "forwarding websocket to durable object",
			actorId,
			path,
		});

		// Validate upgrade
		const upgradeHeader = c.req.header("Upgrade");
		if (!upgradeHeader || upgradeHeader !== "websocket") {
			return new Response("Expected Upgrade: websocket", {
				status: 426,
			});
		}

		const newUrl = new URL(`http://actor${path}`);
		const actorRequest = new Request(newUrl, c.req.raw);

		logger().debug({
			msg: "rewriting websocket url",
			from: c.req.url,
			to: actorRequest.url,
		});

		// Always build fresh request to prevent forwarding unwanted headers
		// HACK: Since we can't build a new request, we need to remove
		// non-standard headers manually
		const headerKeys: string[] = [];
		actorRequest.headers.forEach((v, k) => {
			headerKeys.push(k);
		});
		for (const k of headerKeys) {
			if (!STANDARD_WEBSOCKET_HEADERS.includes(k)) {
				actorRequest.headers.delete(k);
			}
		}

		// Build protocols for WebSocket connection
		const protocols: string[] = [];
		protocols.push(WS_PROTOCOL_STANDARD);
		protocols.push(`${WS_PROTOCOL_TARGET}actor`);
		protocols.push(`${WS_PROTOCOL_ACTOR}${actorId}`);
		protocols.push(`${WS_PROTOCOL_ENCODING}${encoding}`);
		if (params) {
			protocols.push(
				`${WS_PROTOCOL_CONN_PARAMS}${encodeURIComponent(JSON.stringify(params))}`,
			);
		}
		actorRequest.headers.set(
			"sec-websocket-protocol",
			protocols.join(", "),
		);

		const id = c.env.ACTOR_DO.idFromString(actorId);
		const stub = c.env.ACTOR_DO.get(id);

		return await stub.fetch(actorRequest);
	}

	async getForId({
		c,
		actorId,
	}: GetForIdInput<{ Bindings: Bindings }>): Promise<
		ActorOutput | undefined
	> {
		const env = getCloudflareAmbientEnv();

		// Get actor metadata from KV (combined name and key)
		const actorData = (await env.ACTOR_KV.get(
			KEYS.ACTOR.metadata(actorId),
			{
				type: "json",
			},
		)) as ActorData | null;

		// If the actor doesn't exist, return undefined
		if (!actorData) {
			return undefined;
		}

		return {
			actorId,
			name: actorData.name,
			key: actorData.key,
		};
	}

	async getWithKey({
		c,
		name,
		key,
	}: GetWithKeyInput<{ Bindings: Bindings }>): Promise<
		ActorOutput | undefined
	> {
		const env = getCloudflareAmbientEnv();

		logger().debug({ msg: "getWithKey: searching for actor", name, key });

		// Generate deterministic ID from the name and key
		// This is aligned with how createActor generates IDs
		const nameKeyString = serializeNameAndKey(name, key);
		const actorId = env.ACTOR_DO.idFromName(nameKeyString).toString();

		// Check if the actor metadata exists
		const actorData = await env.ACTOR_KV.get(KEYS.ACTOR.metadata(actorId), {
			type: "json",
		});

		if (!actorData) {
			logger().debug({
				msg: "getWithKey: no actor found with matching name and key",
				name,
				key,
				actorId,
			});
			return undefined;
		}

		logger().debug({
			msg: "getWithKey: found actor with matching name and key",
			actorId,
			name,
			key,
		});
		return this.#buildActorOutput(c, actorId);
	}

	async getOrCreateWithKey(
		input: GetOrCreateWithKeyInput,
	): Promise<ActorOutput> {
		// TODO: Prevent race condition here
		const getOutput = await this.getWithKey(input);
		if (getOutput) {
			return getOutput;
		} else {
			return await this.createActor(input);
		}
	}

	async createActor({
		c,
		name,
		key,
		input,
	}: CreateInput<{ Bindings: Bindings }>): Promise<ActorOutput> {
		const env = getCloudflareAmbientEnv();

		// Check if actor with the same name and key already exists
		const existingActor = await this.getWithKey({ c, name, key });
		if (existingActor) {
			throw new ActorAlreadyExists(name, key);
		}

		// Create a deterministic ID from the actor name and key
		// This ensures that actors with the same name and key will have the same ID
		const nameKeyString = serializeNameAndKey(name, key);
		const doId = env.ACTOR_DO.idFromName(nameKeyString);
		const actorId = doId.toString();

		// Init actor
		const actor = env.ACTOR_DO.get(doId);
		await actor.initialize({
			name,
			key,
			input,
		});

		// Store combined actor metadata (name and key)
		const actorData: ActorData = { name, key };
		await env.ACTOR_KV.put(
			KEYS.ACTOR.metadata(actorId),
			JSON.stringify(actorData),
		);

		// Add to key index for lookups by name and key
		await env.ACTOR_KV.put(KEYS.ACTOR.keyIndex(name, key), actorId);

		return {
			actorId,
			name,
			key,
		};
	}

	// Helper method to build actor output from an ID
	async #buildActorOutput(
		c: any,
		actorId: string,
	): Promise<ActorOutput | undefined> {
		const env = getCloudflareAmbientEnv();

		const actorData = (await env.ACTOR_KV.get(
			KEYS.ACTOR.metadata(actorId),
			{
				type: "json",
			},
		)) as ActorData | null;

		if (!actorData) {
			return undefined;
		}

		return {
			actorId,
			name: actorData.name,
			key: actorData.key,
		};
	}

	displayInformation(): ManagerDisplayInformation {
		return {
			name: "Cloudflare Workers",
			properties: {},
		};
	}

	getOrCreateInspectorAccessToken() {
		return generateRandomString();
	}
}
