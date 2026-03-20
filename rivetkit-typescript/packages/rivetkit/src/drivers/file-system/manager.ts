import type { Context as HonoContext } from "hono";
import invariant from "invariant";
import { lookupInRegistry } from "@/actor/definition";
import { ActorStopping } from "@/actor/errors";
import { type ActorRouter, createActorRouter } from "@/actor/router";
import {
	parseWebSocketProtocols,
	routeWebSocket,
} from "@/actor/router-websocket-endpoints";
import { isStaticActorInstance } from "@/actor/instance/mod";
import { createClientWithDriver } from "@/client/client";
import { ClientConfigSchema } from "@/client/config";
import { InlineWebSocketAdapter } from "@/common/inline-websocket-adapter";
import {
	buildHibernatableWebSocketAckStateTestResponse,
	getIndexedWebSocketTestSender,
	parseHibernatableWebSocketAckStateTestRequest,
	registerRemoteHibernatableWebSocketAckHooks,
	setHibernatableWebSocketAckTestHooks,
	setIndexedWebSocketTestSender,
	unregisterRemoteHibernatableWebSocketAckHooks,
	type IndexedWebSocketPayload,
} from "@/common/websocket-test-hooks";
import { noopNext } from "@/common/utils";
import type {
	ActorDriver,
	ActorOutput,
	CreateInput,
	GetForIdInput,
	GetOrCreateWithKeyInput,
	GetWithKeyInput,
	ListActorsInput,
	ManagerDriver,
} from "@/driver-helpers/mod";
import {
	isDynamicActorDefinition,
	createDynamicActorAuthContext,
	createDynamicActorReloadContext,
} from "@/dynamic/internal";
import {
	buildStatusResponse,
	buildStaticStatusResponse,
} from "@/dynamic/runtime-status";
import { timingSafeEqual } from "@/utils/crypto";
import type { ManagerDisplayInformation } from "@/manager/driver";
import type { Encoding, UniversalWebSocket } from "@/mod";
import type { DriverConfig, RegistryConfig } from "@/registry/config";
import type * as schema from "@/schemas/file-system-driver/mod";
import type { GetUpgradeWebSocket } from "@/utils";
import { isDev } from "@/utils/env-vars";
import { VirtualWebSocket } from "@rivetkit/virtual-websocket";
import { createTestWebSocketProxy } from "@/manager/gateway";
import type { FileSystemGlobalState } from "./global-state";
import { logger } from "./log";
import { generateActorId } from "./utils";

const REMOTE_ACK_HOOK_QUERY_PARAM = "__rivetkitAckHook";

const RELOAD_RATE_LIMIT_MAX = 10;
const RELOAD_RATE_LIMIT_WINDOW_MS = 60_000;

export class FileSystemManagerDriver implements ManagerDriver {
	#config: RegistryConfig;
	#state: FileSystemGlobalState;
	#driverConfig: DriverConfig;
	#getUpgradeWebSocket: GetUpgradeWebSocket | undefined;

	#actorDriver: ActorDriver;
	#actorRouter: ActorRouter;

	constructor(
		config: RegistryConfig,
		state: FileSystemGlobalState,
		driverConfig: DriverConfig,
	) {
		this.#config = config;
		this.#state = state;
		this.#driverConfig = driverConfig;

		// Actors run on the same node as the manager, so we create a dummy actor router that we route requests to
		const inlineClient = createClientWithDriver(this);

		this.#actorDriver = this.#driverConfig.actor(
			config,
			this,
			inlineClient,
		);
		this.#actorRouter = createActorRouter(
			this.#config,
			this.#actorDriver,
			undefined,
			config.test.enabled,
		);
	}

	async sendRequest(
		actorId: string,
		actorRequest: Request,
	): Promise<Response> {
		const overlayResponse = await this.#routeOverlayRequest(
			actorId,
			actorRequest,
		);
		if (overlayResponse) {
			return overlayResponse;
		}

		if (await this.#state.isDynamicActor(this.#config, actorId)) {
			await this.#actorDriver.loadActor(actorId);
			return await this.#state.dynamicFetch(actorId, actorRequest);
		}

		return await this.#actorRouter.fetch(actorRequest, {
			actorId,
		});
	}

	async openWebSocket(
		path: string,
		actorId: string,
		encoding: Encoding,
		params: unknown,
	): Promise<UniversalWebSocket> {
		return await this.#openHibernatableWebSocket(
			actorId,
			path,
			encoding,
			params,
		);
	}

	async proxyRequest(
		c: HonoContext,
		actorRequest: Request,
		actorId: string,
	): Promise<Response> {
		const overlayResponse = await this.#routeOverlayRequest(
			actorId,
			actorRequest,
		);
		if (overlayResponse) {
			return overlayResponse;
		}

		if (await this.#state.isDynamicActor(this.#config, actorId)) {
			await this.#actorDriver.loadActor(actorId);
			return await this.#state.dynamicFetch(actorId, actorRequest);
		}

		return await this.#actorRouter.fetch(actorRequest, {
			actorId,
		});
	}

	async proxyWebSocket(
		c: HonoContext,
		path: string,
		actorId: string,
		encoding: Encoding,
		params: unknown,
	): Promise<Response> {
		const upgradeWebSocket = this.#getUpgradeWebSocket?.();
		invariant(upgradeWebSocket, "missing getUpgradeWebSocket");
		const wsHandler = await createTestWebSocketProxy(
			this.#openHibernatableWebSocket(
				actorId,
				path,
				encoding,
				params,
				c.req.raw,
				c.req.header(),
				parseWebSocketProtocols(
					c.req.header("sec-websocket-protocol") ?? undefined,
				).ackHookToken ??
					new URL(c.req.raw.url).searchParams.get(
						REMOTE_ACK_HOOK_QUERY_PARAM,
					) ??
					undefined,
			),
		);
		return upgradeWebSocket(() => wsHandler)(c, noopNext());
	}

	async buildGatewayUrl(actorId: string): Promise<string> {
		const port = this.#config.managerPort ?? 6420;
		return `http://127.0.0.1:${port}/gateway/${encodeURIComponent(actorId)}`;
	}

	async #routeOverlayRequest(
		actorId: string,
		request: Request,
	): Promise<Response | null> {
		const url = new URL(request.url);
		switch (`${request.method} ${url.pathname}`) {
			case "PUT /dynamic/reload":
				return await this.#handleDynamicReloadOverlay(actorId, request);
			case "GET /dynamic/status":
				return await this.#handleDynamicStatusOverlay(actorId, request);
			default:
				return null;
		}
	}

	async #handleDynamicReloadOverlay(
		actorId: string,
		request: Request,
	): Promise<Response> {
		const state = await this.#state.loadActorStateOrError(actorId);
		const definition = lookupInRegistry(this.#config, state.name);
		if (!isDynamicActorDefinition(definition)) {
			return new Response("not a dynamic actor", { status: 404 });
		}

		// Authentication check happens before any state changes.
		const hasAuth = !!definition.auth;
		const hasCanReload = !!definition.canReload;

		if (hasAuth) {
			try {
				const authCtx = createDynamicActorAuthContext(
					this.#state.getInlineClient(),
					actorId,
					state.name,
					state.key as string[],
					this.#state.getActorInitialInput(actorId),
					"unknown",
					request,
				);
				await definition.auth!(authCtx, undefined);
			} catch {
				return new Response("Forbidden", { status: 403 });
			}
		}

		if (hasCanReload) {
			try {
				const reloadCtx = createDynamicActorReloadContext(
					this.#state.getInlineClient(),
					actorId,
					state.name,
					state.key as string[],
					this.#state.getActorInitialInput(actorId),
					"unknown",
					request,
				);
				const allowed = await definition.canReload!(reloadCtx);
				if (!allowed) {
					return new Response("Forbidden", { status: 403 });
				}
			} catch {
				return new Response("Forbidden", { status: 403 });
			}
		}

		if (!hasAuth && !hasCanReload && isDev()) {
			logger().warn({
				msg: "reload allowed without auth or canReload in development mode",
				actorId,
			});
		}

		// Track reload rate for observability (warning-only, not enforcement).
		const dynamicStatus = this.#state.getDynamicStatus(actorId);
		if (dynamicStatus) {
			const now = Date.now();
			if (
				!dynamicStatus.reloadWindowStart ||
				now - dynamicStatus.reloadWindowStart >= RELOAD_RATE_LIMIT_WINDOW_MS
			) {
				dynamicStatus.reloadWindowStart = now;
				dynamicStatus.reloadCount = 1;
			} else {
				dynamicStatus.reloadCount += 1;
			}

			if (dynamicStatus.reloadCount > RELOAD_RATE_LIMIT_MAX) {
				logger().warn({
					msg: "reload rate limit exceeded",
					actorId,
					reloadCount: dynamicStatus.reloadCount,
				});
			}
		}

		await this.#state.reloadDynamicActor(actorId);
		return new Response(null, { status: 200 });
	}

	async #handleDynamicStatusOverlay(
		actorId: string,
		request: Request,
	): Promise<Response> {
		// Inspector-style auth: Bearer token with timing-safe comparison.
		const inspectorToken = this.#config.inspector.token();
		if (isDev() && !inspectorToken) {
			logger().warn({
				msg: "dynamic status endpoint allowed without inspector token in development mode",
				actorId,
			});
		} else {
			const userToken = request.headers
				.get("Authorization")
				?.replace("Bearer ", "");
			if (!userToken || !inspectorToken || !timingSafeEqual(userToken, inspectorToken)) {
				return new Response("Unauthorized", { status: 401 });
			}
		}

		const state = await this.#state.loadActorStateOrError(actorId);
		const definition = lookupInRegistry(this.#config, state.name);

		// Static actors always report as running with generation 0.
		if (!isDynamicActorDefinition(definition)) {
			return Response.json(buildStaticStatusResponse());
		}

		const dynamicStatus = this.#state.getDynamicStatus(actorId);
		if (!dynamicStatus) {
			// No status tracked yet means the actor hasn't been started.
			return Response.json(buildStaticStatusResponse());
		}

		return Response.json(buildStatusResponse(dynamicStatus));
	}

	async getForId({
		actorId,
	}: GetForIdInput): Promise<ActorOutput | undefined> {
		// Validate the actor exists
		const actor = await this.#state.loadActor(actorId);
		if (!actor.state) {
			return undefined;
		}
		if (this.#state.isActorStopping(actorId)) {
			throw new ActorStopping(actorId);
		}

		return actorStateToOutput(actor.state);
	}

	async getWithKey({
		name,
		key,
	}: GetWithKeyInput): Promise<ActorOutput | undefined> {
		// Generate the deterministic actor ID
		const actorId = generateActorId(name, key);

		// Check if actor exists
		const actor = await this.#state.loadActor(actorId);
		if (actor.state) {
			return actorStateToOutput(actor.state);
		}

		return undefined;
	}

	async getOrCreateWithKey(
		input: GetOrCreateWithKeyInput,
	): Promise<ActorOutput> {
		// Generate the deterministic actor ID
		const actorId = generateActorId(input.name, input.key);

		// Use the atomic getOrCreateActor method
		await this.#state.loadOrCreateActor(
			actorId,
			input.name,
			input.key,
			input.input,
		);

		// Start the actor immediately so timestamps are set
		await this.#actorDriver.loadActor(actorId);

		// Reload state to get updated timestamps
		const state = await this.#state.loadActorStateOrError(actorId);
		return actorStateToOutput(state);
	}

	async createActor({ name, key, input }: CreateInput): Promise<ActorOutput> {
		// Generate the deterministic actor ID
		const actorId = generateActorId(name, key);

		await this.#state.createActor(actorId, name, key, input);

		// Start the actor immediately so timestamps are set
		await this.#actorDriver.loadActor(actorId);

		// Reload state to get updated timestamps
		const state = await this.#state.loadActorStateOrError(actorId);
		return actorStateToOutput(state);
	}

	async listActors({ name }: ListActorsInput): Promise<ActorOutput[]> {
		const actors: ActorOutput[] = [];
		const itr = this.#state.getActorsIterator({});

		for await (const actor of itr) {
			if (actor.name === name) {
				actors.push(actorStateToOutput(actor));
			}
		}

		// Sort by create ts desc (most recent first)
		actors.sort((a, b) => {
			const aTs = a.createTs ?? 0;
			const bTs = b.createTs ?? 0;
			return bTs - aTs;
		});

		return actors;
	}

	async kvGet(actorId: string, key: Uint8Array): Promise<string | null> {
		const response = await this.#state.kvBatchGet(actorId, [key]);
		return response[0] !== null
			? new TextDecoder().decode(response[0])
			: null;
	}

	displayInformation(): ManagerDisplayInformation {
		return {
			properties: {
				...(this.#state.persist
					? { Data: this.#state.storagePath }
					: {}),
				Instances: this.#state.actorCountOnStartup.toString(),
			},
		};
	}

	extraStartupLog() {
		return {
			instances: this.#state.actorCountOnStartup,
			data: this.#state.storagePath,
		};
	}

	setGetUpgradeWebSocket(getUpgradeWebSocket: GetUpgradeWebSocket): void {
		this.#getUpgradeWebSocket = getUpgradeWebSocket;
	}

	async #openHibernatableWebSocket(
		actorId: string,
		path: string,
		encoding: Encoding,
		params: unknown,
		requestOverride?: Request,
		headersOverride?: Record<string, string>,
		remoteAckHookToken?: string,
	): Promise<UniversalWebSocket> {
		const { gatewayId, requestId } = createHibernatableRequestMetadata();
		if (await this.#state.isDynamicActor(this.#config, actorId)) {
			return createMockHibernatableWebSocket({
				config: this.#config,
				state: this.#state,
				gatewayId,
				requestId,
				remoteAckHookToken,
				shouldReopenActorWebSocket: () =>
					this.#shouldReopenHibernatableWebSocket(actorId),
				openActorWebSocket: async (
					isRestoringHibernatable,
					_context,
				) => {
					if (isRestoringHibernatable) {
						this.#state.beginHibernatableWebSocketRestore(actorId);
					}
					try {
						await this.#actorDriver.loadActor(actorId);
						const actorWebSocket =
							await this.#state.dynamicOpenWebSocket(
								actorId,
								path,
								encoding,
								params,
								{
									headers: headersOverride,
									gatewayId,
									requestId,
									isHibernatable: true,
									isRestoringHibernatable,
								},
							);
						const sender =
							getIndexedWebSocketTestSender(actorWebSocket);
						invariant(
							sender,
							"dynamic file-system websocket is missing indexed message dispatch support",
						);
						return {
							actorWebSocket,
							sendToActor: sender,
						};
					} finally {
						if (isRestoringHibernatable) {
							this.#state.endHibernatableWebSocketRestore(
								actorId,
							);
						}
					}
				},
			});
		}
		return createMockHibernatableWebSocket({
			config: this.#config,
			state: this.#state,
			gatewayId,
			requestId,
			remoteAckHookToken,
			shouldReopenActorWebSocket: () =>
				this.#shouldReopenHibernatableWebSocket(actorId),
			openActorWebSocket: async (isRestoringHibernatable, context) => {
				if (isRestoringHibernatable) {
					this.#state.beginHibernatableWebSocketRestore(actorId);
				}
				try {
					const normalizedPath = path.startsWith("/")
						? path
						: `/${path}`;
					const request =
						requestOverride ??
						new Request(`http://inline-actor${normalizedPath}`, {
							method: "GET",
						});
					const pathOnly = normalizedPath.split("?")[0];
					const handler = await routeWebSocket(
						request,
						pathOnly,
						headersOverride ?? {},
						this.#config,
						this.#actorDriver,
						actorId,
						encoding,
						params,
						gatewayId,
						requestId,
						true,
						isRestoringHibernatable,
					);
					const shouldPreserveHibernatableConn =
						Boolean(handler.onRestore) &&
						Boolean(handler.conn?.isHibernatable);
					const wrappedHandler = shouldPreserveHibernatableConn
						? {
								...handler,
								onClose: (event: any, wsContext: any) => {
									if (!context.isClientCloseInitiated()) {
										return;
									}
									handler.onClose(event, wsContext);
								},
							}
						: handler;
					const adapter = new InlineWebSocketAdapter(wrappedHandler, {
						restoring: isRestoringHibernatable,
					});
					return {
						actorWebSocket: adapter.clientWebSocket,
						sendToActor: (
							data: IndexedWebSocketPayload,
							rivetMessageIndex?: number,
						) => {
							adapter.dispatchClientMessageWithMetadata(
								data,
								rivetMessageIndex,
							);
							if (
								handler.conn &&
								handler.actor &&
								isStaticActorInstance(handler.actor)
							) {
								handler.actor.handleInboundHibernatableWebSocketMessage(
									handler.conn,
									data as any,
									rivetMessageIndex,
								);
							}
						},
						disconnectActorConn:
							shouldPreserveHibernatableConn && handler.conn
								? async (reason?: string) => {
										await handler.conn?.disconnect(reason);
									}
								: undefined,
						markActorConnStale:
							shouldPreserveHibernatableConn && handler.conn
								? () => {
										invariant(
											handler.actor &&
												isStaticActorInstance(
													handler.actor,
												),
											"missing static actor for stale hibernatable websocket cleanup",
										);
										invariant(
											handler.conn,
											"missing hibernatable connection for stale websocket cleanup",
										);
										handler.actor.connectionManager.detachPersistedHibernatableConnDriver(
											handler.conn,
											"file-system-driver.stale-client-close",
										);
									}
								: undefined,
					};
				} finally {
					if (isRestoringHibernatable) {
						this.#state.endHibernatableWebSocketRestore(actorId);
					}
				}
			},
		});
	}

	#shouldReopenHibernatableWebSocket(actorId: string): boolean {
		if (this.#state.isActorStopping(actorId)) {
			return true;
		}

		try {
			return (
				this.#state.getActorOrError(actorId).actor?.isStopping ?? true
			);
		} catch {
			return true;
		}
	}
}

function actorStateToOutput(state: schema.ActorState): ActorOutput {
	return {
		actorId: state.actorId,
		name: state.name,
		key: state.key as string[],
		createTs: Number(state.createdAt),
		startTs: state.startTs !== null ? Number(state.startTs) : null,
		connectableTs:
			state.connectableTs !== null ? Number(state.connectableTs) : null,
		sleepTs: state.sleepTs !== null ? Number(state.sleepTs) : null,
		destroyTs: state.destroyTs !== null ? Number(state.destroyTs) : null,
	};
}

function createHibernatableRequestMetadata(): {
	gatewayId: ArrayBuffer;
	requestId: ArrayBuffer;
} {
	const gatewayId = new Uint8Array(4);
	const requestId = new Uint8Array(4);
	crypto.getRandomValues(gatewayId);
	crypto.getRandomValues(requestId);
	return {
		gatewayId: gatewayId.buffer.slice(0),
		requestId: requestId.buffer.slice(0),
	};
}

function createMockHibernatableWebSocket(input: {
	config: RegistryConfig;
	state: FileSystemGlobalState;
	gatewayId: ArrayBuffer;
	requestId: ArrayBuffer;
	remoteAckHookToken?: string;
	shouldReopenActorWebSocket: () => boolean;
	openActorWebSocket: (
		isRestoringHibernatable: boolean,
		context: {
			isClientCloseInitiated: () => boolean;
		},
	) => Promise<{
		actorWebSocket: UniversalWebSocket;
		sendToActor: (
			data: IndexedWebSocketPayload,
			rivetMessageIndex?: number,
		) => void | Promise<void>;
		disconnectActorConn?: (reason?: string) => Promise<void>;
		markActorConnStale?: () => void;
	}>;
}): UniversalWebSocket {
	const {
		config,
		state,
		gatewayId,
		requestId,
		remoteAckHookToken,
		shouldReopenActorWebSocket,
		openActorWebSocket,
	} = input;
	let readyState: 0 | 1 | 2 | 3 = 0;
	let nextServerMessageIndex = 1;
	let lastSentIndex = 0;
	let lastAckedIndex = 0;
	let hasOpened = false;
	let closeInitiatedByClient = false;
	let openingActorWebSocket:
		| Promise<{
				actorWebSocket: UniversalWebSocket;
				sendToActor: (
					data: IndexedWebSocketPayload,
					rivetMessageIndex?: number,
				) => void | Promise<void>;
				disconnectActorConn?: (reason?: string) => Promise<void>;
				markActorConnStale?: () => void;
		  }>
		| undefined;
	let currentActorWebSocket: UniversalWebSocket | undefined;
	let currentDisconnectActorConn:
		| ((reason?: string) => Promise<void>)
		| undefined;
	let currentMarkActorConnStale: (() => void) | undefined;
	let currentSendToActor:
		| ((
				data: IndexedWebSocketPayload,
				rivetMessageIndex?: number,
		  ) => void | Promise<void>)
		| undefined;
	let flushPendingMessagesPromise: Promise<void> | undefined;
	const pendingIndexes: number[] = [];
	const pendingMessages: Array<{
		data: IndexedWebSocketPayload;
		rivetMessageIndex: number;
	}> = [];
	const ackWaiters = new Map<number, Array<() => void>>();
	let observerRegistered = true;

	const unregisterObserver = () => {
		if (!observerRegistered) {
			return;
		}
		observerRegistered = false;
		state.unregisterHibernatableWebSocketAckObserver(gatewayId, requestId);
		unregisterRemoteHibernatableWebSocketAckHooks(
			remoteAckHookToken,
			config.test.enabled,
		);
	};

	const bindActorWebSocket = (actorWebSocket: UniversalWebSocket) => {
		currentActorWebSocket = actorWebSocket;
		const schedulePendingFlush = () => {
			queueMicrotask(() => {
				void flushPendingMessages().catch((error) => {
					logger().debug({
						msg: "mock hibernatable websocket flush failed",
						error,
					});
				});
			});
		};

		actorWebSocket.addEventListener("open", () => {
			if (readyState >= 2) {
				return;
			}
			if (!hasOpened) {
				hasOpened = true;
				readyState = 1;
				clientWebSocket.triggerOpen();
			}
			schedulePendingFlush();
		});
		actorWebSocket.addEventListener("message", (event: any) => {
			clientWebSocket.triggerMessage(event.data);
		});
		actorWebSocket.addEventListener("close", (event: any) => {
			if (currentActorWebSocket === actorWebSocket) {
				currentActorWebSocket = undefined;
				currentDisconnectActorConn = undefined;
				currentMarkActorConnStale = undefined;
				currentSendToActor = undefined;
			}
			if (!closeInitiatedByClient && readyState < 2) {
				return;
			}
			readyState = 3;
			unregisterObserver();
			clientWebSocket.triggerClose(event.code, event.reason);
		});
		actorWebSocket.addEventListener("error", (error: unknown) => {
			clientWebSocket.triggerError(error);
		});

		if (actorWebSocket.readyState === 1) {
			if (!hasOpened) {
				hasOpened = true;
				readyState = 1;
				clientWebSocket.triggerOpen();
			}
			schedulePendingFlush();
		}
	};

	const ensureActorWebSocket = async (): Promise<void> => {
		if (readyState >= 2) {
			return;
		}
		const shouldRestoreActorWebSocket =
			hasOpened && shouldReopenActorWebSocket();
		if (
			currentActorWebSocket?.readyState === 1 &&
			currentSendToActor &&
			!shouldRestoreActorWebSocket
		) {
			return;
		}
		if (shouldRestoreActorWebSocket) {
			currentActorWebSocket = undefined;
			currentSendToActor = undefined;
		}
		if (!openingActorWebSocket) {
			logger().debug({
				msg: "mock hibernatable websocket opening actor websocket",
				shouldRestoreActorWebSocket,
			});
			openingActorWebSocket = openActorWebSocket(
				shouldRestoreActorWebSocket,
				{
					isClientCloseInitiated: () => closeInitiatedByClient,
				},
			)
				.then((binding) => {
					logger().debug({
						msg: "mock hibernatable websocket actor websocket ready",
						shouldRestoreActorWebSocket,
						actorReadyState: binding.actorWebSocket.readyState,
					});
					currentDisconnectActorConn = binding.disconnectActorConn;
					currentMarkActorConnStale = binding.markActorConnStale;
					currentSendToActor = binding.sendToActor;
					bindActorWebSocket(binding.actorWebSocket);
					return binding;
				})
				.finally(() => {
					openingActorWebSocket = undefined;
				});
		}
		await openingActorWebSocket;
	};

	const flushPendingMessages = async () => {
		if (flushPendingMessagesPromise) {
			await flushPendingMessagesPromise;
			return;
		}

		flushPendingMessagesPromise = (async () => {
			await ensureActorWebSocket();
			logger().debug({
				msg: "mock hibernatable websocket flush state",
				pendingMessageCount: pendingMessages.length,
				hasSender: Boolean(currentSendToActor),
				currentActorReadyState: currentActorWebSocket?.readyState,
			});
			while (
				pendingMessages.length > 0 &&
				currentSendToActor &&
				currentActorWebSocket &&
				currentActorWebSocket.readyState === currentActorWebSocket.OPEN
			) {
				const next = pendingMessages.shift();
				if (!next) {
					return;
				}
				logger().debug({
					msg: "mock hibernatable websocket delivering pending message",
					rivetMessageIndex: next.rivetMessageIndex,
				});
				const sendResult = currentSendToActor(
					next.data,
					next.rivetMessageIndex,
				);
				if (sendResult && typeof sendResult.then === "function") {
					void sendResult.catch((error: unknown) => {
						logger().debug({
							msg: "mock hibernatable websocket pending send failed",
							error,
						});
					});
				}
			}
		})().finally(() => {
			flushPendingMessagesPromise = undefined;
		});

		await flushPendingMessagesPromise;
	};

	const resolveAckWaiters = (serverMessageIndex: number) => {
		for (const [index, waiters] of ackWaiters) {
			if (index > serverMessageIndex) {
				continue;
			}
			ackWaiters.delete(index);
			for (const resolve of waiters) {
				resolve();
			}
		}
	};

	const enqueueMessage = (
		data: IndexedWebSocketPayload,
		rivetMessageIndex: number,
	) => {
		lastSentIndex = rivetMessageIndex;
		pendingIndexes.push(rivetMessageIndex);
		pendingMessages.push({ data, rivetMessageIndex });
		void flushPendingMessages();
	};

	const clientWebSocket = new VirtualWebSocket({
		getReadyState: () => readyState,
		onSend: (data) => {
			const ackStateRequest =
				parseHibernatableWebSocketAckStateTestRequest(
					data,
					config.test.enabled,
				);
			if (ackStateRequest) {
				const response = buildHibernatableWebSocketAckStateTestResponse(
					{
						lastSentIndex,
						lastAckedIndex,
						pendingIndexes: [...pendingIndexes],
					},
					config.test.enabled,
				);
				invariant(
					response,
					"missing hibernatable websocket ack test response",
				);
				clientWebSocket.triggerMessage(response);
				return;
			}
			const rivetMessageIndex = nextServerMessageIndex;
			nextServerMessageIndex += 1;
			enqueueMessage(data, rivetMessageIndex);
		},
		onClose: (code, reason) => {
			readyState = 2;
			closeInitiatedByClient = true;
			unregisterObserver();
			void (async () => {
				if (shouldReopenActorWebSocket() && currentMarkActorConnStale) {
					// Keep the persisted conn metadata in place when the client
					// disappears during sleep so the next wake can reconcile it as
					// a stale hibernatable request.
					currentMarkActorConnStale();
					currentActorWebSocket = undefined;
					currentDisconnectActorConn = undefined;
					currentMarkActorConnStale = undefined;
					currentSendToActor = undefined;
					readyState = 3;
					return;
				}
				if (currentDisconnectActorConn) {
					await currentDisconnectActorConn(reason);
				}
				if (
					currentActorWebSocket &&
					currentActorWebSocket.readyState !==
						currentActorWebSocket.CLOSED &&
					currentActorWebSocket.readyState !==
						currentActorWebSocket.CLOSING
				) {
					currentActorWebSocket.close(code, reason);
					return;
				}
				readyState = 3;
			})().catch((error) => {
				logger().debug({
					msg: "failed to close mock hibernatable websocket actor connection",
					error,
				});
				readyState = 3;
			});
		},
	});

	state.registerHibernatableWebSocketAckObserver(gatewayId, requestId, {
		onAck: (serverMessageIndex) => {
			lastAckedIndex = Math.max(lastAckedIndex, serverMessageIndex);
			while (
				pendingIndexes.length > 0 &&
				pendingIndexes[0] <= serverMessageIndex
			) {
				pendingIndexes.shift();
			}
			resolveAckWaiters(serverMessageIndex);
		},
	});

	setIndexedWebSocketTestSender(
		clientWebSocket,
		(data, rivetMessageIndex) => {
			const indexedMessage =
				typeof rivetMessageIndex === "number"
					? rivetMessageIndex
					: nextServerMessageIndex;
			nextServerMessageIndex = Math.max(
				nextServerMessageIndex,
				indexedMessage + 1,
			);
			enqueueMessage(data, indexedMessage);
		},
		config.test.enabled,
	);
	setHibernatableWebSocketAckTestHooks(
		clientWebSocket,
		{
			getState: () => ({
				lastSentIndex,
				lastAckedIndex,
				pendingIndexes: [...pendingIndexes],
			}),
			waitForAck: async (serverMessageIndex) => {
				if (lastAckedIndex >= serverMessageIndex) {
					return;
				}
				await new Promise<void>((resolve) => {
					const existing = ackWaiters.get(serverMessageIndex) ?? [];
					existing.push(resolve);
					ackWaiters.set(serverMessageIndex, existing);
				});
			},
		},
		config.test.enabled,
	);
	if (remoteAckHookToken) {
		registerRemoteHibernatableWebSocketAckHooks(
			remoteAckHookToken,
			{
				getState: () => ({
					lastSentIndex,
					lastAckedIndex,
					pendingIndexes: [...pendingIndexes],
				}),
				waitForAck: async (serverMessageIndex) => {
					if (lastAckedIndex >= serverMessageIndex) {
						return;
					}
					await new Promise<void>((resolve) => {
						const existing =
							ackWaiters.get(serverMessageIndex) ?? [];
						existing.push(resolve);
						ackWaiters.set(serverMessageIndex, existing);
					});
				},
			},
			config.test.enabled,
		);
	}

	void ensureActorWebSocket();

	return clientWebSocket;
}
