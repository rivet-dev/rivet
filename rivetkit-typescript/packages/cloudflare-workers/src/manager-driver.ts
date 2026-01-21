import type { Hono, Context as HonoContext } from "hono";
import type { Encoding, RegistryConfig, UniversalWebSocket } from "rivetkit";
import {
	type ActorOutput,
	type CreateInput,
	type GetForIdInput,
	type GetOrCreateWithKeyInput,
	type GetWithKeyInput,
	type ListActorsInput,
	type ManagerDisplayInformation,
	type ManagerDriver,
	WS_PROTOCOL_ACTOR,
	WS_PROTOCOL_CONN_PARAMS,
	WS_PROTOCOL_ENCODING,
	WS_PROTOCOL_STANDARD,
	WS_PROTOCOL_TARGET,
} from "rivetkit/driver-helpers";
import {
	ActorDuplicateKey,
	ActorNotFound,
	InternalError,
} from "rivetkit/errors";
import { assertUnreachable } from "rivetkit/utils";
import { parseActorId } from "./actor-id";
import { getCloudflareAmbientEnv } from "./handler";
import { logger } from "./log";
import type { Bindings } from "./mod";
import { serializeNameAndKey } from "./util";

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

		// Parse actor ID to get DO ID
		const [doId] = parseActorId(actorId);

		logger().debug({
			msg: "sending request to durable object",
			actorId,
			doId,
			method: actorRequest.method,
			url: actorRequest.url,
		});

		const id = env.ACTOR_DO.idFromString(doId);
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

		// Parse actor ID to get DO ID
		const [doId] = parseActorId(actorId);

		logger().debug({
			msg: "opening websocket to durable object",
			actorId,
			doId,
			path,
		});

		// Make a fetch request to the Durable Object with WebSocket upgrade
		const id = env.ACTOR_DO.idFromString(doId);
		const stub = env.ACTOR_DO.get(id);

		const protocols: string[] = [];
		protocols.push(WS_PROTOCOL_STANDARD);
		protocols.push(`${WS_PROTOCOL_TARGET}actor`);
		protocols.push(`${WS_PROTOCOL_ACTOR}${encodeURIComponent(actorId)}`);
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

	async buildGatewayUrl(actorId: string): Promise<string> {
		return `http://actor/gateway/${encodeURIComponent(actorId)}`;
	}

	async proxyRequest(
		c: HonoContext<{ Bindings: Bindings }>,
		actorRequest: Request,
		actorId: string,
	): Promise<Response> {
		const env = getCloudflareAmbientEnv();

		// Parse actor ID to get DO ID
		const [doId] = parseActorId(actorId);

		logger().debug({
			msg: "forwarding request to durable object",
			actorId,
			doId,
			method: actorRequest.method,
			url: actorRequest.url,
		});

		const id = env.ACTOR_DO.idFromString(doId);
		const stub = env.ACTOR_DO.get(id);

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
		protocols.push(`${WS_PROTOCOL_ACTOR}${encodeURIComponent(actorId)}`);
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

		// Parse actor ID to get DO ID
		const env = getCloudflareAmbientEnv();
		const [doId] = parseActorId(actorId);
		const id = env.ACTOR_DO.idFromString(doId);
		const stub = env.ACTOR_DO.get(id);

		return await stub.fetch(actorRequest);
	}

	async getForId({
		c,
		name,
		actorId,
	}: GetForIdInput<{ Bindings: Bindings }>): Promise<
		ActorOutput | undefined
	> {
		const env = getCloudflareAmbientEnv();

		// Parse actor ID to get DO ID and expected generation
		const [doId, expectedGeneration] = parseActorId(actorId);

		// Get the Durable Object stub
		const id = env.ACTOR_DO.idFromString(doId);
		const stub = env.ACTOR_DO.get(id);

		// Call the DO's getMetadata method
		const result = await stub.getMetadata();

		if (!result) {
			logger().debug({
				msg: "getForId: actor not found",
				actorId,
			});
			return undefined;
		}

		// Check if the actor IDs match in order to check if the generation matches
		if (result.actorId !== actorId) {
			logger().debug({
				msg: "getForId: generation mismatch",
				requestedActorId: actorId,
				actualActorId: result.actorId,
			});
			return undefined;
		}

		if (result.destroying) {
			throw new ActorNotFound(actorId);
		}

		return {
			actorId: result.actorId,
			name: result.name,
			key: result.key,
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
		const nameKeyString = serializeNameAndKey(name, key);
		const doId = env.ACTOR_DO.idFromName(nameKeyString).toString();

		// Try to get the Durable Object to see if it exists
		const id = env.ACTOR_DO.idFromString(doId);
		const stub = env.ACTOR_DO.get(id);

		// Check if actor exists without creating it
		const result = await stub.getMetadata();

		if (result) {
			logger().debug({
				msg: "getWithKey: found actor with matching name and key",
				actorId: result.actorId,
				name: result.name,
				key: result.key,
			});
			return {
				actorId: result.actorId,
				name: result.name,
				key: result.key,
			};
		} else {
			logger().debug({
				msg: "getWithKey: no actor found with matching name and key",
				name,
				key,
				doId,
			});
			return undefined;
		}
	}

	async getOrCreateWithKey({
		c,
		name,
		key,
		input,
	}: GetOrCreateWithKeyInput<{ Bindings: Bindings }>): Promise<ActorOutput> {
		const env = getCloudflareAmbientEnv();

		// Create a deterministic ID from the actor name and key
		// This ensures that actors with the same name and key will have the same ID
		const nameKeyString = serializeNameAndKey(name, key);
		const doId = env.ACTOR_DO.idFromName(nameKeyString);

		// Get or create actor using the Durable Object's method
		const actor = env.ACTOR_DO.get(doId);
		const result = await actor.create({
			name,
			key,
			input,
			allowExisting: true,
		});
		if ("success" in result) {
			const { actorId, created } = result.success;
			logger().debug({
				msg: "getOrCreateWithKey result",
				actorId,
				name,
				key,
				created,
			});

			return {
				actorId,
				name,
				key,
			};
		} else if ("error" in result) {
			throw new Error(`Error: ${JSON.stringify(result.error)}`);
		} else {
			assertUnreachable(result);
		}
	}

	async createActor({
		c,
		name,
		key,
		input,
	}: CreateInput<{ Bindings: Bindings }>): Promise<ActorOutput> {
		const env = getCloudflareAmbientEnv();

		// Create a deterministic ID from the actor name and key
		// This ensures that actors with the same name and key will have the same ID
		const nameKeyString = serializeNameAndKey(name, key);
		const doId = env.ACTOR_DO.idFromName(nameKeyString);

		// Create actor - this will fail if it already exists
		const actor = env.ACTOR_DO.get(doId);
		const result = await actor.create({
			name,
			key,
			input,
			allowExisting: false,
		});

		if ("success" in result) {
			const { actorId } = result.success;
			return {
				actorId,
				name,
				key,
			};
		} else if ("error" in result) {
			if (result.error.actorAlreadyExists) {
				throw new ActorDuplicateKey(name, key);
			}

			throw new InternalError(
				`Unknown error creating actor: ${JSON.stringify(result.error)}`,
			);
		} else {
			assertUnreachable(result);
		}
	}

	async listActors({ c, name }: ListActorsInput): Promise<ActorOutput[]> {
		logger().warn({
			msg: "listActors not fully implemented for Cloudflare Workers",
			name,
		});
		return [];
	}

	displayInformation(): ManagerDisplayInformation {
		return {
			properties: {
				Driver: "Cloudflare Workers",
			},
		};
	}

	setGetUpgradeWebSocket(): void {
		// No-op for Cloudflare Workers - WebSocket upgrades are handled by the DO
	}

	async kvGet(actorId: string, key: Uint8Array): Promise<string | null> {
		const env = getCloudflareAmbientEnv();

		// Parse actor ID to get DO ID
		const [doId] = parseActorId(actorId);

		const id = env.ACTOR_DO.idFromString(doId);
		const stub = env.ACTOR_DO.get(id);

		const value = await stub.managerKvGet(key);
		return value !== null ? new TextDecoder().decode(value) : null;
	}
}
