import type { Context as HonoContext, Next } from "hono";
import type { WSContext } from "hono/ws";
import invariant from "invariant";
import { MissingActorHeader, WebSocketsNotEnabled } from "@/actor/errors";
import {
	parseWebSocketProtocols,
	type UpgradeWebSocketArgs,
} from "@/actor/router-websocket-endpoints";
import {
	HEADER_RIVET_ACTOR,
	HEADER_RIVET_TARGET,
	WS_PROTOCOL_ACTOR,
	WS_PROTOCOL_TARGET,
} from "@/common/actor-router-consts";
import type { UniversalWebSocket } from "@/mod";
import type { RegistryConfig } from "@/registry/config";
import { promiseWithResolvers } from "@/utils";
import type { GetUpgradeWebSocket } from "@/utils";
import type { EngineControlClient } from "@/engine-client/driver";
import { parseActorPath } from "./actor-path";
import { logger } from "./log";
import { resolvePathBasedActorPath } from "./resolve-query";

// Re-export types used by tests and other consumers
export type {
	ParsedActorPath,
	ParsedDirectActorPath,
	ParsedQueryActorPath,
} from "./actor-path";
export { parseActorPath } from "./actor-path";

/**
 * Handle path-based WebSocket routing
 */
async function handleWebSocketGatewayPathBased(
	config: RegistryConfig,
	engineClient: EngineControlClient,
	c: HonoContext,
	actorPathInfo: ReturnType<typeof parseActorPath> & {},
	getUpgradeWebSocket: GetUpgradeWebSocket | undefined,
): Promise<Response> {
	const upgradeWebSocket = getUpgradeWebSocket?.();
	if (!upgradeWebSocket) {
		throw new WebSocketsNotEnabled();
	}

	const resolvedActorPathInfo = await resolvePathBasedActorPath(
		config,
		engineClient,
		c,
		actorPathInfo,
	);

	// NOTE: Token validation implemented in EE

	// Parse additional configuration from Sec-WebSocket-Protocol header
	const { encoding, connParams } = parseWebSocketProtocols(
		c.req.header("sec-websocket-protocol"),
	);

	logger().debug({
		msg: "proxying websocket to actor via path-based routing",
		actorId: resolvedActorPathInfo.actorId,
		path: resolvedActorPathInfo.remainingPath,
		encoding,
	});

	return await engineClient.proxyWebSocket(
		c,
		resolvedActorPathInfo.remainingPath,
		resolvedActorPathInfo.actorId,
		encoding as any, // Will be validated by driver
		connParams,
	);
}

/**
 * Handle path-based HTTP routing
 */
async function handleHttpGatewayPathBased(
	config: RegistryConfig,
	engineClient: EngineControlClient,
	c: HonoContext,
	actorPathInfo: ReturnType<typeof parseActorPath> & {},
): Promise<Response> {
	const resolvedActorPathInfo = await resolvePathBasedActorPath(
		config,
		engineClient,
		c,
		actorPathInfo,
	);

	// NOTE: Token validation implemented in EE

	logger().debug({
		msg: "proxying request to actor via path-based routing",
		actorId: resolvedActorPathInfo.actorId,
		path: resolvedActorPathInfo.remainingPath,
		method: c.req.method,
	});

	// Preserve all headers
	const proxyHeaders = new Headers(c.req.raw.headers);

	// Build the proxy request with the actor URL format
	const proxyUrl = new URL(
		`http://actor${resolvedActorPathInfo.remainingPath}`,
	);

	const proxyRequest = new Request(proxyUrl, {
		method: c.req.raw.method,
		headers: proxyHeaders,
		body: c.req.raw.body,
		signal: c.req.raw.signal,
		duplex: "half",
	} as RequestInit);

	return await engineClient.proxyRequest(
		c,
		proxyRequest,
		resolvedActorPathInfo.actorId,
	);
}

/**
 * Provides an endpoint to connect to individual actors.
 *
 * Routes requests using either path-based routing or header-based routing:
 *
 * Path-based routing (checked first):
 * - /gateway/{actor_id}/{...path}
 * - /gateway/{actor_id}@{token}/{...path}
 * - /gateway/{name}/{...path}?rvt-namespace={namespace}&rvt-method={get|getOrCreate}&...
 *
 * Header-based routing (fallback):
 * - WebSocket requests: Uses sec-websocket-protocol for routing (target.actor, actor.{id})
 * - HTTP requests: Uses x-rivet-target and x-rivet-actor headers for routing
 */
export async function actorGateway(
	config: RegistryConfig,
	engineClient: EngineControlClient,
	getUpgradeWebSocket: GetUpgradeWebSocket | undefined,
	c: HonoContext,
	next: Next,
) {
	// Skip test routes - let them be handled by their specific handlers
	if (c.req.path.startsWith("/.test/")) {
		return next();
	}

	// Strip basePath from the request path
	let strippedPath = c.req.path;
	if (
		config.managerBasePath &&
		strippedPath.startsWith(config.managerBasePath)
	) {
		strippedPath = strippedPath.slice(config.managerBasePath.length);
		// Ensure the path starts with /
		if (!strippedPath.startsWith("/")) {
			strippedPath = `/${strippedPath}`;
		}
	}

	// Include query string if present (needed for parseActorPath to preserve query params)
	const pathWithQuery = c.req.url.includes("?")
		? strippedPath + c.req.url.substring(c.req.url.indexOf("?"))
		: strippedPath;

	// First, check if this is an actor path-based route
	const actorPathInfo = parseActorPath(pathWithQuery);
	if (actorPathInfo) {
		logger().debug({
			msg: "routing using path-based actor routing",
			actorPathInfo,
		});

		// Check if this is a WebSocket upgrade request
		const isWebSocket = c.req.header("upgrade") === "websocket";

		if (isWebSocket) {
			return await handleWebSocketGatewayPathBased(
				config,
				engineClient,
				c,
				actorPathInfo,
				getUpgradeWebSocket,
			);
		}

		// Handle regular HTTP requests
		return await handleHttpGatewayPathBased(
			config,
			engineClient,
			c,
			actorPathInfo,
		);
	}

	// Fallback to header-based routing
	// Check if this is a WebSocket upgrade request
	if (c.req.header("upgrade") === "websocket") {
		return await handleWebSocketGateway(
			config,
			engineClient,
			getUpgradeWebSocket,
			c,
			strippedPath,
		);
	}

	// Handle regular HTTP requests
	return await handleHttpGateway(engineClient, c, next, strippedPath);
}

/**
 * Handle WebSocket requests using sec-websocket-protocol for routing
 */
async function handleWebSocketGateway(
	_config: RegistryConfig,
	engineClient: EngineControlClient,
	getUpgradeWebSocket: GetUpgradeWebSocket | undefined,
	c: HonoContext,
	strippedPath: string,
) {
	const upgradeWebSocket = getUpgradeWebSocket?.();
	if (!upgradeWebSocket) {
		throw new WebSocketsNotEnabled();
	}

	// Parse target and actor ID from Sec-WebSocket-Protocol header
	const protocolsHeader = c.req.header("sec-websocket-protocol");
	const protocols = protocolsHeader?.split(",").map((p) => p.trim()) ?? [];
	const target = protocols
		.find((p) => p.startsWith(WS_PROTOCOL_TARGET))
		?.slice(WS_PROTOCOL_TARGET.length);
	const actorId = protocols
		.find((p) => p.startsWith(WS_PROTOCOL_ACTOR))
		?.slice(WS_PROTOCOL_ACTOR.length);

	// Parse encoding and connection params from protocols
	const { encoding, connParams } = parseWebSocketProtocols(protocolsHeader);

	if (target !== "actor") {
		return c.text("WebSocket upgrade requires target.actor protocol", 400);
	}

	if (!actorId) {
		throw new MissingActorHeader();
	}

	logger().debug({
		msg: "proxying websocket to actor",
		actorId,
		path: strippedPath,
		encoding,
	});

	// Include query string if present
	const pathWithQuery = c.req.url.includes("?")
		? strippedPath + c.req.url.substring(c.req.url.indexOf("?"))
		: strippedPath;

	return await engineClient.proxyWebSocket(
		c,
		pathWithQuery,
		actorId,
		encoding,
		connParams,
	);
}

/**
 * Handle HTTP requests using x-rivet headers for routing
 */
async function handleHttpGateway(
	engineClient: EngineControlClient,
	c: HonoContext,
	next: Next,
	strippedPath: string,
) {
	const target = c.req.header(HEADER_RIVET_TARGET);
	const actorId = c.req.header(HEADER_RIVET_ACTOR);

	if (target !== "actor") {
		return next();
	}

	if (!actorId) {
		throw new MissingActorHeader();
	}

	logger().debug({
		msg: "proxying request to actor",
		actorId,
		path: strippedPath,
		method: c.req.method,
	});

	// Preserve all headers except the routing headers
	const proxyHeaders = new Headers(c.req.raw.headers);
	proxyHeaders.delete(HEADER_RIVET_TARGET);
	proxyHeaders.delete(HEADER_RIVET_ACTOR);

	// Build the proxy request with the actor URL format
	const url = new URL(c.req.url);
	const proxyUrl = new URL(`http://actor${strippedPath}${url.search}`);

	const proxyRequest = new Request(proxyUrl, {
		method: c.req.raw.method,
		headers: proxyHeaders,
		body: c.req.raw.body,
		signal: c.req.raw.signal,
		duplex: "half",
	} as RequestInit);

	return await engineClient.proxyRequest(c, proxyRequest, actorId);
}

/**
 * Creates a WebSocket proxy for test endpoints that forwards messages between server and client WebSockets
 *
 * clientToProxyWs = the websocket from the client -> the proxy
 * proxyToActorWs = the websocket from the proxy -> the actor
 */
export async function createTestWebSocketProxy(
	proxyToActorWsPromise: Promise<UniversalWebSocket>,
): Promise<UpgradeWebSocketArgs> {
	// Store a reference to the resolved WebSocket
	let proxyToActorWs: UniversalWebSocket | null = null;
	const {
		promise: clientToProxyWsPromise,
		resolve: clientToProxyWsResolve,
		reject: clientToProxyWsReject,
	} = promiseWithResolvers<WSContext>((reason) =>
		logger().warn({
			msg: "unhandled client websocket promise rejection",
			reason,
		}),
	);
	try {
		// Resolve the client WebSocket promise
		logger().debug({ msg: "awaiting client websocket promise" });
		proxyToActorWs = await proxyToActorWsPromise;
		logger().debug({
			msg: "client websocket promise resolved",
			constructor: proxyToActorWs?.constructor.name,
		});

		// Wait for ws to open
		await new Promise<void>((resolve, reject) => {
			invariant(proxyToActorWs, "missing proxyToActorWs");

			const onOpen = () => {
				logger().debug({
					msg: "test websocket connection to actor opened",
				});
				resolve();
			};
			const onError = (error: any) => {
				logger().error({
					msg: "test websocket connection failed",
					error,
				});
				reject(
					new Error(
						`Failed to open WebSocket: ${error.message || error}`,
					),
				);
				clientToProxyWsReject();
			};

			proxyToActorWs.addEventListener("open", onOpen);

			proxyToActorWs.addEventListener("error", onError);

			proxyToActorWs.addEventListener(
				"message",
				async (clientEvt: MessageEvent) => {
					const clientToProxyWs = await clientToProxyWsPromise;

					logger().debug({
						msg: `test websocket connection message from client`,
						dataType: typeof clientEvt.data,
						isBlob: clientEvt.data instanceof Blob,
						isArrayBuffer: clientEvt.data instanceof ArrayBuffer,
						dataConstructor: clientEvt.data?.constructor?.name,
						dataStr:
							typeof clientEvt.data === "string"
								? clientEvt.data.substring(0, 100)
								: undefined,
					});

					if (clientToProxyWs.readyState === 1) {
						// OPEN
						// Handle Blob data
						if (clientEvt.data instanceof Blob) {
							clientEvt.data
								.arrayBuffer()
								.then((buffer) => {
									logger().debug({
										msg: "converted client blob to arraybuffer, sending to server",
										bufferSize: buffer.byteLength,
									});
									clientToProxyWs.send(buffer as any);
								})
								.catch((error) => {
									logger().error({
										msg: "failed to convert blob to arraybuffer",
										error,
									});
								});
						} else {
							logger().debug({
								msg: "sending client data directly to server",
								dataType: typeof clientEvt.data,
								dataLength:
									typeof clientEvt.data === "string"
										? clientEvt.data.length
										: undefined,
							});
							clientToProxyWs.send(clientEvt.data as any);
						}
					}
				},
			);

			proxyToActorWs.addEventListener("close", async (clientEvt: any) => {
				const clientToProxyWs = await clientToProxyWsPromise;

				logger().debug({
					msg: `test websocket connection closed`,
				});

				if (clientToProxyWs.readyState !== 3) {
					// Not CLOSED
					clientToProxyWs.close(clientEvt.code, clientEvt.reason);
				}
			});

			proxyToActorWs.addEventListener("error", async () => {
				const clientToProxyWs = await clientToProxyWsPromise;

				logger().debug({
					msg: `test websocket connection error`,
				});

				if (clientToProxyWs.readyState !== 3) {
					// Not CLOSED
					clientToProxyWs.close(1011, "Error in client websocket");
				}
			});
		});
	} catch (error) {
		logger().error({
			msg: `failed to establish client websocket connection`,
			error,
		});
		return {
			onOpen: (_evt, clientToProxyWs) => {
				clientToProxyWs.close(1011, "Failed to establish connection");
			},
			onMessage: () => {},
			onError: () => {},
			onClose: () => {},
		};
	}

	// Create WebSocket proxy handlers to relay messages between client and server
	return {
		onOpen: (_evt: any, clientToProxyWs: WSContext) => {
			logger().debug({
				msg: `test websocket connection from client opened`,
			});

			// Check WebSocket type
			logger().debug({
				msg: "proxyToActorWs info",
				constructor: proxyToActorWs.constructor.name,
				hasAddEventListener:
					typeof proxyToActorWs.addEventListener === "function",
				readyState: proxyToActorWs.readyState,
			});

			clientToProxyWsResolve(clientToProxyWs);
		},
		onMessage: (evt: { data: any }) => {
			logger().debug({
				msg: "received message from server",
				dataType: typeof evt.data,
				isBlob: evt.data instanceof Blob,
				isArrayBuffer: evt.data instanceof ArrayBuffer,
				dataConstructor: evt.data?.constructor?.name,
				dataStr:
					typeof evt.data === "string"
						? evt.data.substring(0, 100)
						: undefined,
			});

			// Forward messages from server websocket to client websocket
			if (proxyToActorWs.readyState === 1) {
				// OPEN
				// Handle Blob data
				if (evt.data instanceof Blob) {
					evt.data
						.arrayBuffer()
						.then((buffer) => {
							logger().debug({
								msg: "converted blob to arraybuffer, sending",
								bufferSize: buffer.byteLength,
							});
							proxyToActorWs.send(buffer);
						})
						.catch((error) => {
							logger().error({
								msg: "failed to convert blob to arraybuffer",
								error,
							});
						});
				} else {
					logger().debug({
						msg: "sending data directly",
						dataType: typeof evt.data,
						dataLength:
							typeof evt.data === "string"
								? evt.data.length
								: undefined,
					});
					proxyToActorWs.send(evt.data);
				}
			}
		},
		onClose: (
			event: {
				wasClean: boolean;
				code: number;
				reason: string;
			},
			clientToProxyWs: WSContext,
		) => {
			logger().debug({
				msg: `server websocket closed`,
				wasClean: event.wasClean,
				code: event.code,
				reason: event.reason,
			});

			// HACK: Close socket in order to fix bug with Cloudflare leaving WS in closing state
			// https://github.com/cloudflare/workerd/issues/2569
			clientToProxyWs.close(1000, "hack_force_close");

			// Close the client websocket when the server websocket closes
			if (
				proxyToActorWs &&
				proxyToActorWs.readyState !== proxyToActorWs.CLOSED &&
				proxyToActorWs.readyState !== proxyToActorWs.CLOSING
			) {
				// Don't pass code/message since this may affect how close events are triggered
				proxyToActorWs.close(1000, event.reason);
			}
		},
		onError: (error: unknown) => {
			logger().error({
				msg: `error in server websocket`,
				error,
			});

			// Close the client websocket on error
			if (
				proxyToActorWs &&
				proxyToActorWs.readyState !== proxyToActorWs.CLOSED &&
				proxyToActorWs.readyState !== proxyToActorWs.CLOSING
			) {
				proxyToActorWs.close(1011, "Error in server websocket");
			}

			clientToProxyWsReject();
		},
	};
}
