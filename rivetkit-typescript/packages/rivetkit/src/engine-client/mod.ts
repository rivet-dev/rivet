import type { Context as HonoContext } from "hono";
import invariant from "invariant";
import { deserializeActorKey, serializeActorKey } from "@/actor/keys";
import type { ClientConfig } from "@/client/client";
import type { IssuedToken, IssueTokenOptions } from "@/client/auth";
import {
	isInvalidToken,
	isInvalidTokenResponse,
	TokenProvider,
} from "@/client/token-provider";
import {
	PATH_CONNECT,
	PATH_WEBSOCKET_BASE,
	PATH_WEBSOCKET_PREFIX,
} from "@/common/actor-router-consts";
import type { JsonCompatValue } from "@/common/encoding";
import { noopNext } from "@/common/utils";
import type { Actor as ApiActor } from "@/engine-api/actors";
import type {
	ActorOutput,
	CreateInput,
	EngineControlClient,
	GatewayRequestOptions,
	GatewayTarget,
	GetForIdInput,
	GetOrCreateWithKeyInput,
	GetWithKeyInput,
	ListActorsInput,
	RuntimeDisplayInformation,
} from "@/engine-client/driver";
import { shouldSkipReadyWait } from "@/engine-client/driver";
import type { Encoding, UniversalWebSocket } from "@/mod";
import { encodeCborCompat, uint8ArrayToBase64 } from "@/serde";
import { combineUrlPath, type GetUpgradeWebSocket } from "@/utils";
import { getNextPhase } from "@/utils/env-vars";
import { sendHttpRequestToGateway } from "./actor-http-client";
import {
	buildActorGatewayUrl,
	buildActorQueryGatewayUrl,
	buildWebSocketProtocols,
	openWebSocketToGateway,
} from "./actor-websocket-client";
import {
	createActor,
	destroyActor,
	getActor,
	getActorByKey,
	getOrCreateActor,
	issueToken as issueEngineToken,
	listActorsByName,
} from "./api-endpoints";
import { EngineApiError, getEndpoint } from "./api-utils";
import { logger } from "./log";
import { lookupMetadataCached } from "./metadata";
import { createWebSocketProxy } from "./ws-proxy";

export class RemoteEngineControlClient implements EngineControlClient {
	#config: ClientConfig;
	#metadataPromise: Promise<void> | undefined;
	#tokenProvider?: TokenProvider;
	#webSocketTokens = new WeakMap<
		UniversalWebSocket,
		{
			refresh?: Promise<string>;
		}
	>();

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
		if (runConfig.getToken)
			this.#tokenProvider = new TokenProvider(runConfig.getToken);

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
						if (metadataData.clientToken && !this.#tokenProvider) {
							this.#config.token = metadataData.clientToken;
						}

						logger().info({
							msg: "overriding client endpoint",
							endpoint: metadataData.clientEndpoint,
							namespace: metadataData.clientNamespace,
						});
					}

					logger().info({
						msg: "connected to rivetkit runtime",
						runtime: metadataData.runtime,
						version: metadataData.version,
						envoy: metadataData.envoy,
					});
				},
			);
		}
	}

	async #authorizedConfig(): Promise<ClientConfig> {
		if (!this.#tokenProvider) return this.#config;
		return { ...this.#config, token: await this.#tokenProvider.current() };
	}

	async #withCredential<T>(
		request: (config: ClientConfig) => Promise<T>,
		readOnly = false,
	): Promise<T> {
		const config = await this.#authorizedConfig();
		try {
			return await request(config);
		} catch (error) {
			if (
				this.#tokenProvider &&
				config.token &&
				error instanceof EngineApiError &&
				error.statusCode === 401 &&
				isInvalidToken(error.group, error.code)
			) {
				if (readOnly) {
					const token = await this.#tokenProvider.refreshIfCurrent(
						config.token,
					);
					return request({ ...this.#config, token });
				}
				// Mutations may have executed; refresh the next call, not this one.
				await this.#tokenProvider.refreshIfCurrent(config.token);
			}
			throw error;
		}
	}

	async issueToken(options: IssueTokenOptions): Promise<IssuedToken> {
		await this.#metadataPromise;
		const response = await this.#withCredential((config) =>
			issueEngineToken(config, options),
		);
		return {
			token: response.token,
			issuedAt: response.issued_ts,
			expiresAt: response.expires_ts,
		};
	}

	async getForId({
		name,
		actorId,
	}: GetForIdInput): Promise<ActorOutput | undefined> {
		await this.#metadataPromise;

		// Fetch from API if not in cache
		const response = await this.#withCredential(
			(config) => getActor(config, name, actorId),
			true,
		);
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
		name,
		key,
	}: GetWithKeyInput): Promise<ActorOutput | undefined> {
		await this.#metadataPromise;

		logger().debug({ msg: "getWithKey: searching for actor", name, key });

		// If not in local cache, fetch by key from API
		try {
			const response = await this.#withCredential(
				(config) => getActorByKey(config, name, key),
				true,
			);
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
		await this.#metadataPromise;

		const {
			name,
			key,
			input: actorInput,
			region,
			crashPolicy,
			poolName,
		} = input;

		logger().info({
			msg: "getOrCreateWithKey: getting or creating actor via engine api",
			name,
			key,
		});

		try {
			const { actor, created } = await this.#withCredential((config) =>
				getOrCreateActor(config, {
					datacenter: region,
					name,
					key: serializeActorKey(key),
					runner_name_selector: poolName ?? this.#config.poolName,
					input: actorInput
						? uint8ArrayToBase64(
								encodeCborCompat(actorInput as JsonCompatValue),
							)
						: undefined,
					crash_policy: crashPolicy ?? "sleep",
				}),
			);

			logger().info({
				msg: "getOrCreateWithKey: actor ready",
				actorId: actor.actor_id,
				name,
				key,
				created,
			});

			return apiActorToOutput(actor);
		} catch (error) {
			// The key is reserved in a different datacenter, which means the
			// actor already exists there. get-by-key forwards to the reserved
			// datacenter, so retry as a get to resolve the existing actor.
			if (
				error instanceof EngineApiError &&
				error.group === "actor" &&
				error.code === "key_reserved_in_different_datacenter"
			) {
				logger().warn({
					msg: "getOrCreateWithKey: key reserved in different datacenter, retrying as get",
					name,
					key,
				});

				const response = await this.#withCredential(
					(config) => getActorByKey(config, name, key),
					true,
				);
				const existing = response.actors[0];
				if (!existing) throw error;

				logger().info({
					msg: "getOrCreateWithKey: resolved existing actor via get",
					actorId: existing.actor_id,
					name,
					key,
				});

				return apiActorToOutput(existing);
			}
			throw error;
		}
	}

	async createActor({
		name,
		key,
		input,
		region,
		crashPolicy,
		poolName,
	}: CreateInput): Promise<ActorOutput> {
		await this.#metadataPromise;

		logger().info({ msg: "creating actor via engine api", name, key });

		// Create actor via engine API
		const result = await this.#withCredential((config) =>
			createActor(config, {
				datacenter: region,
				name,
				runner_name_selector: poolName ?? this.#config.poolName,
				key: serializeActorKey(key),
				input: input
					? uint8ArrayToBase64(
							encodeCborCompat(input as JsonCompatValue),
						)
					: undefined,
				crash_policy: crashPolicy ?? "sleep",
			}),
		);

		logger().info({
			msg: "actor created",
			actorId: result.actor.actor_id,
			name,
			key,
		});

		return apiActorToOutput(result.actor);
	}

	async listActors({ name }: ListActorsInput): Promise<ActorOutput[]> {
		await this.#metadataPromise;

		logger().debug({ msg: "listing actors via engine api", name });

		const response = await this.#withCredential(
			(config) => listActorsByName(config, name),
			true,
		);

		return response.actors.map(apiActorToOutput);
	}

	async destroyActor(actorId: string): Promise<void> {
		await this.#metadataPromise;

		logger().info({ msg: "destroying actor via engine api", actorId });

		await this.#withCredential((config) => destroyActor(config, actorId));

		logger().info({ msg: "actor destroyed", actorId });
	}

	async sendRequest(
		target: GatewayTarget,
		actorRequest: Request,
		options: GatewayRequestOptions = {},
	): Promise<Response> {
		await this.#metadataPromise;
		const config = await this.#authorizedConfig();
		const httpOptions = {
			...options,
			directActorId: shouldSkipReadyWait(options)
				? directActorIdFromTarget(target)
				: undefined,
		};

		const send = (current: ClientConfig) =>
			sendHttpRequestToGateway(
				current,
				this.#buildGatewayUrlForTarget(
					current,
					target,
					requestPath(actorRequest),
					options,
				),
				actorRequest,
				httpOptions,
			);
		const response = await send(config);
		if (
			!this.#tokenProvider ||
			!config.token ||
			!isInvalidTokenResponse(response)
		)
			return response;
		if (
			(actorRequest.method === "GET" || actorRequest.method === "HEAD") &&
			!("getOrCreateForKey" in target)
		) {
			const token = await this.#tokenProvider.refreshIfCurrent(
				config.token,
			);
			await response.body?.cancel();
			return send({ ...this.#config, token });
		}
		await this.#tokenProvider.refreshIfCurrent(config.token);
		return response;
	}

	async openWebSocket(
		path: string,
		target: GatewayTarget,
		encoding: Encoding,
		params: unknown,
		options: GatewayRequestOptions = {},
	): Promise<UniversalWebSocket> {
		await this.#metadataPromise;
		const config = await this.#authorizedConfig();

		const gatewayUrl = this.#buildGatewayUrlForTarget(
			config,
			target,
			path,
			options,
		);

		const ws = await openWebSocketToGateway(
			config,
			gatewayUrl,
			encoding,
			params,
			{
				...options,
				directActorId: shouldSkipReadyWait(options)
					? directActorIdFromTarget(target)
					: undefined,
			},
		);
		this.#watchWebSocketAuth(ws, config.token);
		return ws;
	}

	async refreshAuthToken(ws: UniversalWebSocket): Promise<boolean> {
		const context = this.#webSocketTokens.get(ws);
		if (!context?.refresh) return false;
		await context.refresh;
		return true;
	}

	#watchWebSocketAuth(ws: UniversalWebSocket, token?: string): void {
		const tokenProvider = this.#tokenProvider;
		if (!tokenProvider || !token) return;
		const context: { refresh?: Promise<string> } = {};
		this.#webSocketTokens.set(ws, context);
		ws.addEventListener(
			"close",
			(event: { code?: number; reason?: string }) => {
				if (event.code !== 1008 || !event.reason) return;
				const separator = event.reason.indexOf(".");
				if (separator < 0) return;
				const code = event.reason
					.slice(separator + 1)
					.split(/[\s:#]/, 1)[0];
				if (isInvalidToken(event.reason.slice(0, separator), code)) {
					context.refresh = tokenProvider
						.refreshIfCurrent(token)
						.catch((error) => {
							logger().warn(
								"failed to renew authentication token",
							);
							throw error;
						});
					// Raw websockets have no built-in reconnection; preempt unhandled rejection.
					context.refresh.catch(() => {});
				}
			},
		);
	}

	async buildGatewayUrl(
		target: GatewayTarget,
		options: GatewayRequestOptions = {},
	): Promise<string> {
		await this.#metadataPromise;
		return this.#buildGatewayUrlForTarget(
			await this.#authorizedConfig(),
			target,
			"",
			options,
		);
	}

	async proxyRequest(
		_c: HonoContext,
		actorRequest: Request,
		actorId: string,
	): Promise<Response> {
		return this.sendRequest({ directId: actorId }, actorRequest);
	}

	async proxyWebSocket(
		c: HonoContext,
		path: string,
		actorId: string,
		encoding: Encoding,
		params: unknown,
	): Promise<Response> {
		await this.#metadataPromise;
		const config = await this.#authorizedConfig();

		const upgradeWebSocket = config.getUpgradeWebSocket?.();
		invariant(upgradeWebSocket, "missing getUpgradeWebSocket");

		const endpoint = getEndpoint(config);
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
			config,
			encoding,
			params,
			undefined,
			{
				target: "actor",
				actorId,
			},
		);
		const args = await createWebSocketProxy(c, wsGuardUrl, protocols);

		return await upgradeWebSocket(() => args)(c, noopNext());
	}

	displayInformation(): RuntimeDisplayInformation {
		return { properties: {} };
	}

	setGetUpgradeWebSocket(getUpgradeWebSocket: GetUpgradeWebSocket): void {
		this.#config.getUpgradeWebSocket = getUpgradeWebSocket;
	}

	#buildGatewayUrlForTarget(
		config: ClientConfig,
		target: GatewayTarget,
		path: string,
		options: GatewayRequestOptions = {},
	): string {
		const endpoint = getEndpoint(config);

		if (
			shouldSkipReadyWait(options) &&
			directActorIdFromTarget(target) &&
			canUseDirectSkipReadyWaitPath(path)
		) {
			return combineUrlPath(endpoint, path);
		}

		if ("directId" in target) {
			return buildActorGatewayUrl(
				endpoint,
				target.directId,
				config.token,
				path,
			);
		}

		if ("getForId" in target) {
			return buildActorGatewayUrl(
				endpoint,
				target.getForId.actorId,
				config.token,
				path,
			);
		}

		if ("getForKey" in target || "getOrCreateForKey" in target) {
			return buildActorQueryGatewayUrl(
				endpoint,
				config.namespace,
				target,
				config.token,
				path,
				config.maxInputSize,
				undefined,
				"getOrCreateForKey" in target
					? (target.getOrCreateForKey.poolName ?? config.poolName)
					: undefined,
				options,
			);
		}

		if ("create" in target) {
			throw new Error(
				"Gateway URLs only support direct actor IDs, get, and getOrCreate targets.",
			);
		}

		throw new Error("unreachable: unknown gateway target type");
	}
}

function canUseDirectSkipReadyWaitPath(path: string): boolean {
	return (
		isActorHttpRequestPath(path) ||
		isPathOrQuery(path, PATH_CONNECT) ||
		isPathOrQuery(path, PATH_WEBSOCKET_BASE) ||
		path.startsWith(PATH_WEBSOCKET_PREFIX)
	);
}

function isPathOrQuery(path: string, basePath: string): boolean {
	return path === basePath || path.startsWith(`${basePath}?`);
}

function isActorHttpRequestPath(path: string): boolean {
	const stripped = path.slice("/request".length);
	return (
		path.startsWith("/request") &&
		(stripped.length === 0 ||
			stripped.startsWith("/") ||
			stripped.startsWith("?"))
	);
}

function directActorIdFromTarget(target: GatewayTarget): string | undefined {
	if ("directId" in target) {
		return target.directId;
	}
	if ("getForId" in target) {
		return target.getForId.actorId;
	}
	return undefined;
}

function requestPath(req: Request): string {
	const url = new URL(req.url);
	return `${url.pathname}${url.search}`;
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
