import * as cbor from "cbor-x";
import type { Context as HonoContext } from "hono";
import invariant from "invariant";
import { deserializeActorKey, serializeActorKey } from "@/actor/keys";
import type { ClientConfig } from "@/client/client";
import { noopNext } from "@/common/utils";
import type {
	ActorOutput,
	CreateInput,
	GetForIdInput,
	GetOrCreateWithKeyInput,
	GetWithKeyInput,
	ListActorsInput,
	ManagerDisplayInformation,
	ManagerDriver,
} from "@/driver-helpers/mod";
import type { Actor as ApiActor } from "@/manager-api/actors";
import type { Encoding, UniversalWebSocket } from "@/mod";
import { uint8ArrayToBase64 } from "@/serde";
import { combineUrlPath, type GetUpgradeWebSocket } from "@/utils";
import { getNextPhase } from "@/utils/env-vars";
import { sendHttpRequestToActor } from "./actor-http-client";
import {
	buildActorGatewayUrl,
	buildWebSocketProtocols,
	openWebSocketToActor,
} from "./actor-websocket-client";
import {
	createActor,
	destroyActor,
	getActor,
	getActorByKey,
	getOrCreateActor,
	kvGet,
	listActorsByName,
} from "./api-endpoints";
import { EngineApiError, getEndpoint } from "./api-utils";
import { logger } from "./log";
import { lookupMetadataCached } from "./metadata";
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

export class RemoteManagerDriver implements ManagerDriver {
	#config: ClientConfig;
	#metadataPromise: Promise<void> | undefined;

	constructor(runConfig: ClientConfig) {
		// Disable health check if in Next.js build phase since there is no `/metadata` endpoint
		//
		// See https://github.com/vercel/next.js/blob/5e6b008b561caf2710ab7be63320a3d549474a5b/packages/next/shared/lib/constants.ts#L19-L23
		if (getNextPhase() === "phase-production-build") {
			logger().info(
				"detected next.js build phase, disabling health check",
			);
			runConfig.disableMetadataLookup = true;
		}

		// Clone config so we can mutate the endpoint in #metadataPromise
		// NOTE: This is a shallow clone, so mutating nested properties will not do anything
		this.#config = { ...runConfig };

		// Perform metadata check if enabled
		if (!runConfig.disableMetadataLookup) {
			// This should never error, since it uses pRetry. If it does for
			// any reason, we'll surface the error anywhere #metadataPromise is
			// awaited.
			this.#metadataPromise = lookupMetadataCached(this.#config).then(
				(metadataData) => {
					// Override endpoint for all future requests
					if (metadataData.clientEndpoint) {
						this.#config.endpoint = metadataData.clientEndpoint;
						if (metadataData.clientNamespace) {
							this.#config.namespace =
								metadataData.clientNamespace;
						}
						if (metadataData.clientToken) {
							this.#config.token = metadataData.clientToken;
						}

						logger().info({
							msg: "overriding client endpoint",
							endpoint: metadataData.clientEndpoint,
							namespace: metadataData.clientNamespace,
							token: metadataData.clientToken,
						});
					}

					logger().info({
						msg: "connected to rivetkit manager",
						runtime: metadataData.runtime,
						version: metadataData.version,
						runner: metadataData.runner,
					});
				},
			);
		}
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

		return apiActorToOutput(actor);
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

			logger().debug({
				msg: "getWithKey: found actor via api",
				actorId: actor.actor_id,
				name,
				key,
			});

			return apiActorToOutput(actor);
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

		logger().info({
			msg: "getOrCreateWithKey: actor ready",
			actorId: actor.actor_id,
			name,
			key,
			created,
		});

		return apiActorToOutput(actor);
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

		logger().info({
			msg: "actor created",
			actorId: result.actor.actor_id,
			name,
			key,
		});

		return apiActorToOutput(result.actor);
	}

	async listActors({ c, name }: ListActorsInput): Promise<ActorOutput[]> {
		// Wait for metadata check to complete if in progress
		if (this.#metadataPromise) {
			await this.#metadataPromise;
		}

		logger().debug({ msg: "listing actors via engine api", name });

		const response = await listActorsByName(this.#config, name);

		return response.actors.map(apiActorToOutput);
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

	async buildGatewayUrl(actorId: string): Promise<string> {
		if (this.#metadataPromise) {
			await this.#metadataPromise;
		}

		const endpoint = getEndpoint(this.#config);
		return buildActorGatewayUrl(endpoint, actorId, this.#config.token);
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

	async kvGet(actorId: string, key: Uint8Array): Promise<string | null> {
		// Wait for metadata check to complete if in progress
		if (this.#metadataPromise) {
			await this.#metadataPromise;
		}

		logger().debug({ msg: "getting kv value via engine api", key });

		const response = await kvGet(
			this.#config,
			actorId,
			new TextDecoder("utf8").decode(key),
		);

		return response.value;
	}

	displayInformation(): ManagerDisplayInformation {
		return { properties: {} };
	}

	setGetUpgradeWebSocket(getUpgradeWebSocket: GetUpgradeWebSocket): void {
		this.#config.getUpgradeWebSocket = getUpgradeWebSocket;
	}
}

function apiActorToOutput(actor: ApiActor): ActorOutput {
	return {
		actorId: actor.actor_id,
		name: actor.name,
		key: deserializeActorKey(actor.key),
		createTs: actor.create_ts,
		startTs: actor.start_ts ?? null,
		connectableTs: actor.connectable_ts ?? null,
		sleepTs: actor.sleep_ts ?? null,
		destroyTs: actor.destroy_ts ?? null,
		error: actor.error ?? undefined,
	};
}
