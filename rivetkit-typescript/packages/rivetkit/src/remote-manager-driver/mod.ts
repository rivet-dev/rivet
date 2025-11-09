import * as cbor from "cbor-x";
import type { Context as HonoContext } from "hono";
import invariant from "invariant";
import { deserializeActorKey, serializeActorKey } from "@/actor/keys";
import { generateRandomString } from "@/actor/utils";
import type { ClientConfig } from "@/client/client";
import { noopNext, stringifyError } from "@/common/utils";
import type {
	ActorOutput,
	CreateInput,
	GetForIdInput,
	GetOrCreateWithKeyInput,
	GetWithKeyInput,
	ManagerDisplayInformation,
	ManagerDriver,
} from "@/driver-helpers/mod";
import type { Encoding, UniversalWebSocket } from "@/mod";
import { uint8ArrayToBase64 } from "@/serde";
import { combineUrlPath, getEnvUniversal } from "@/utils";
import { sendHttpRequestToActor } from "./actor-http-client";
import {
	buildWebSocketProtocols,
	openWebSocketToActor,
} from "./actor-websocket-client";
import {
	createActor,
	destroyActor,
	getActor,
	getActorByKey,
	getMetadata,
	getOrCreateActor,
} from "./api-endpoints";
import { EngineApiError, getEndpoint } from "./api-utils";
import { logger } from "./log";
import { createWebSocketProxy } from "./ws-proxy";

// TODO:
// // Lazily import the dynamic imports so we don't have to turn `createClient` in to an async fn
// const dynamicImports = (async () => {
// 	// Import dynamic dependencies
// 	const [WebSocket, EventSource] = await Promise.all([
// 		importWebSocket(),
// 		importEventSource(),
// 	]);
// 	return {
// 		WebSocket,
// 		EventSource,
// 	};
// })();

// Global cache to store metadata check promises for each endpoint
const metadataCheckCache = new Map<string, Promise<void>>();

export class RemoteManagerDriver implements ManagerDriver {
	#config: ClientConfig;
	#metadataPromise: Promise<void> | undefined;

	constructor(runConfig: ClientConfig) {
		// Disable health check if in Next.js build phase since there is no `/metadata` endpoint
		//
		// See https://github.com/vercel/next.js/blob/5e6b008b561caf2710ab7be63320a3d549474a5b/packages/next/shared/lib/constants.ts#L19-L23
		if (getEnvUniversal("NEXT_PHASE") === "phase-production-build") {
			logger().info(
				"detected next.js build phase, disabling health check",
			);
			runConfig.disableHealthCheck = true;
		}

		this.#config = runConfig;

		// Perform metadata check if enabled
		if (!runConfig.disableHealthCheck) {
			this.#metadataPromise = this.#performMetadataCheck(runConfig);
			this.#metadataPromise.catch((error) => {
				logger().error({
					msg: "metadata check failed",
					error:
						error instanceof Error ? error.message : String(error),
				});
			});
		}
	}

	async #performMetadataCheck(config: ClientConfig): Promise<void> {
		const endpoint = getEndpoint(config);

		// Check if metadata check is already in progress or completed for this endpoint
		const existingPromise = metadataCheckCache.get(endpoint);
		if (existingPromise) {
			return existingPromise;
		}

		// Create and store the promise immediately to prevent racing requests
		const metadataCheckPromise = (async () => {
			try {
				const metadataData = await getMetadata(config);

				if (metadataData.clientEndpoint) {
					logger().info({
						msg: "received new client endpoint from metadata",
						endpoint: metadataData.clientEndpoint,
					});
					this.#config.endpoint = metadataData.clientEndpoint;
				}

				// Log successful metadata check with runtime and version info
				logger().info({
					msg: "connected to rivetkit manager",
					runtime: metadataData.runtime,
					version: metadataData.version,
					runner: metadataData.runner,
				});
			} catch (error) {
				logger().error({
					msg: "health check failed, validate the Rivet endpoint is configured correctly",
					endpoint,
					error: stringifyError(error),
				});
			}
		})();

		metadataCheckCache.set(endpoint, metadataCheckPromise);
		return metadataCheckPromise;
	}

	async getForId({
		c,
		name,
		actorId,
	}: GetForIdInput): Promise<ActorOutput | undefined> {
		// Wait for metadata check to complete if in progress
		if (this.#metadataPromise) {
			await this.#metadataPromise;
		}

		// Fetch from API if not in cache
		const response = await getActor(this.#config, name, actorId);
		const actor = response.actors[0];
		if (!actor) return undefined;

		// Validate name matches
		if (actor.name !== name) {
			logger().debug({
				msg: "actor name mismatch from api",
				actorId,
				apiName: actor.name,
				requestedName: name,
			});
			return undefined;
		}

		const keyRaw = actor.key;
		invariant(keyRaw, `actor ${actorId} should have key`);
		const key = deserializeActorKey(keyRaw);

		return {
			actorId,
			name,
			key,
		};
	}

	async getWithKey({
		c,
		name,
		key,
	}: GetWithKeyInput): Promise<ActorOutput | undefined> {
		// Wait for metadata check to complete if in progress
		if (this.#metadataPromise) {
			await this.#metadataPromise;
		}

		logger().debug({ msg: "getWithKey: searching for actor", name, key });

		// If not in local cache, fetch by key from API
		try {
			const response = await getActorByKey(this.#config, name, key);
			const actor = response.actors[0];
			if (!actor) return undefined;

			const actorId = actor.actor_id;

			logger().debug({
				msg: "getWithKey: found actor via api",
				actorId,
				name,
				key,
			});

			return {
				actorId,
				name,
				key,
			};
		} catch (error) {
			if (
				error instanceof EngineApiError &&
				(error as EngineApiError).group === "actor" &&
				(error as EngineApiError).code === "not_found"
			) {
				return undefined;
			}
			throw error;
		}
	}

	async getOrCreateWithKey(
		input: GetOrCreateWithKeyInput,
	): Promise<ActorOutput> {
		// Wait for metadata check to complete if in progress
		if (this.#metadataPromise) {
			await this.#metadataPromise;
		}

		const { c, name, key, input: actorInput, region } = input;

		logger().info({
			msg: "getOrCreateWithKey: getting or creating actor via engine api",
			name,
			key,
		});

		const { actor, created } = await getOrCreateActor(this.#config, {
			datacenter: region,
			name,
			key: serializeActorKey(key),
			runner_name_selector: this.#config.runnerName,
			input: actorInput
				? uint8ArrayToBase64(cbor.encode(actorInput))
				: undefined,
			crash_policy: "sleep",
		});

		const actorId = actor.actor_id;

		logger().info({
			msg: "getOrCreateWithKey: actor ready",
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
	}

	async createActor({
		c,
		name,
		key,
		input,
		region,
	}: CreateInput): Promise<ActorOutput> {
		// Wait for metadata check to complete if in progress
		if (this.#metadataPromise) {
			await this.#metadataPromise;
		}

		logger().info({ msg: "creating actor via engine api", name, key });

		// Create actor via engine API
		const result = await createActor(this.#config, {
			datacenter: region,
			name,
			runner_name_selector: this.#config.runnerName,
			key: serializeActorKey(key),
			input: input ? uint8ArrayToBase64(cbor.encode(input)) : undefined,
			crash_policy: "sleep",
		});
		const actorId = result.actor.actor_id;

		logger().info({ msg: "actor created", actorId, name, key });

		return {
			actorId,
			name,
			key,
		};
	}

	async destroyActor(actorId: string): Promise<void> {
		// Wait for metadata check to complete if in progress
		if (this.#metadataPromise) {
			await this.#metadataPromise;
		}

		logger().info({ msg: "destroying actor via engine api", actorId });

		await destroyActor(this.#config, actorId);

		logger().info({ msg: "actor destroyed", actorId });
	}

	async sendRequest(
		actorId: string,
		actorRequest: Request,
	): Promise<Response> {
		// Wait for metadata check to complete if in progress
		if (this.#metadataPromise) {
			await this.#metadataPromise;
		}

		return await sendHttpRequestToActor(
			this.#config,
			actorId,
			actorRequest,
		);
	}

	async openWebSocket(
		path: string,
		actorId: string,
		encoding: Encoding,
		params: unknown,
	): Promise<UniversalWebSocket> {
		// Wait for metadata check to complete if in progress
		if (this.#metadataPromise) {
			await this.#metadataPromise;
		}

		return await openWebSocketToActor(
			this.#config,
			path,
			actorId,
			encoding,
			params,
		);
	}

	async proxyRequest(
		_c: HonoContext,
		actorRequest: Request,
		actorId: string,
	): Promise<Response> {
		// Wait for metadata check to complete if in progress
		if (this.#metadataPromise) {
			await this.#metadataPromise;
		}

		return await sendHttpRequestToActor(
			this.#config,
			actorId,
			actorRequest,
		);
	}

	async proxyWebSocket(
		c: HonoContext,
		path: string,
		actorId: string,
		encoding: Encoding,
		params: unknown,
	): Promise<Response> {
		// Wait for metadata check to complete if in progress
		if (this.#metadataPromise) {
			await this.#metadataPromise;
		}

		const upgradeWebSocket = this.#config.getUpgradeWebSocket?.();
		invariant(upgradeWebSocket, "missing getUpgradeWebSocket");

		const endpoint = getEndpoint(this.#config);
		const guardUrl = combineUrlPath(endpoint, path);
		const wsGuardUrl = guardUrl.replace("http://", "ws://");

		logger().debug({
			msg: "forwarding websocket to actor via guard",
			actorId,
			path,
			guardUrl,
		});

		// Build protocols
		const protocols = buildWebSocketProtocols(
			this.#config,
			encoding,
			params,
		);
		const args = await createWebSocketProxy(c, wsGuardUrl, protocols);

		return await upgradeWebSocket(() => args)(c, noopNext());
	}

	displayInformation(): ManagerDisplayInformation {
		return { name: "Remote", properties: {} };
	}

	getOrCreateInspectorAccessToken() {
		return generateRandomString();
	}
}
