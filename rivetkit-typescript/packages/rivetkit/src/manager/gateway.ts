import type { Context as HonoContext, Next } from "hono";
import type { WSContext } from "hono/ws";
import { MissingActorHeader, WebSocketsNotEnabled } from "@/actor/errors";
import type { UpgradeWebSocketArgs } from "@/actor/router-websocket-endpoints";
import {
	HEADER_RIVET_ACTOR,
	HEADER_RIVET_TARGET,
	WS_PROTOCOL_ACTOR,
	WS_PROTOCOL_CONN_PARAMS,
	WS_PROTOCOL_ENCODING,
	WS_PROTOCOL_TARGET,
} from "@/common/actor-router-consts";
import type { UniversalWebSocket } from "@/mod";
import type { RunnerConfig } from "@/registry/run-config";
import { promiseWithResolvers } from "@/utils";
import type { ManagerDriver } from "./driver";
import { logger } from "./log";

interface ActorPathInfo {
	actorId: string;
	token?: string;
	remainingPath: string;
}

/**
 * Handle path-based WebSocket routing
 */
async function handleWebSocketGatewayPathBased(
	runConfig: RunnerConfig,
	managerDriver: ManagerDriver,
	c: HonoContext,
	actorPathInfo: ActorPathInfo,
): Promise<Response> {
	const upgradeWebSocket = runConfig.getUpgradeWebSocket?.();
	if (!upgradeWebSocket) {
		throw new WebSocketsNotEnabled();
	}

	// NOTE: Token validation implemented in EE

	// Parse additional configuration from Sec-WebSocket-Protocol header
	const protocols = c.req.header("sec-websocket-protocol");
	let encodingRaw: string | undefined;
	let connParamsRaw: string | undefined;

	if (protocols) {
		const protocolList = protocols.split(",").map((p) => p.trim());
		for (const protocol of protocolList) {
			if (protocol.startsWith(WS_PROTOCOL_ENCODING)) {
				encodingRaw = protocol.substring(WS_PROTOCOL_ENCODING.length);
			} else if (protocol.startsWith(WS_PROTOCOL_CONN_PARAMS)) {
				connParamsRaw = decodeURIComponent(
					protocol.substring(WS_PROTOCOL_CONN_PARAMS.length),
				);
			}
		}
	}

	logger().debug({
		msg: "proxying websocket to actor via path-based routing",
		actorId: actorPathInfo.actorId,
		path: actorPathInfo.remainingPath,
		encoding: encodingRaw,
	});

	const encoding = encodingRaw || "json";
	const connParams = connParamsRaw ? JSON.parse(connParamsRaw) : undefined;

	return await managerDriver.proxyWebSocket(
		c,
		actorPathInfo.remainingPath,
		actorPathInfo.actorId,
		encoding as any, // Will be validated by driver
		connParams,
	);
}

/**
 * Handle path-based HTTP routing
 */
async function handleHttpGatewayPathBased(
	managerDriver: ManagerDriver,
	c: HonoContext,
	actorPathInfo: ActorPathInfo,
): Promise<Response> {
	// NOTE: Token validation implemented in EE

	logger().debug({
		msg: "proxying request to actor via path-based routing",
		actorId: actorPathInfo.actorId,
		path: actorPathInfo.remainingPath,
		method: c.req.method,
	});

	// Preserve all headers
	const proxyHeaders = new Headers(c.req.raw.headers);

	// Build the proxy request with the actor URL format
	const proxyUrl = new URL(`http://actor${actorPathInfo.remainingPath}`);

	const proxyRequest = new Request(proxyUrl, {
		method: c.req.raw.method,
		headers: proxyHeaders,
		body: c.req.raw.body,
		signal: c.req.raw.signal,
		duplex: "half",
	} as RequestInit);

	return await managerDriver.proxyRequest(
		c,
		proxyRequest,
		actorPathInfo.actorId,
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
 *
 * Header-based routing (fallback):
 * - WebSocket requests: Uses sec-websocket-protocol for routing (target.actor, actor.{id})
 * - HTTP requests: Uses x-rivet-target and x-rivet-actor headers for routing
 */
export async function actorGateway(
	runConfig: RunnerConfig,
	managerDriver: ManagerDriver,
	c: HonoContext,
	next: Next,
) {
	// Skip test routes - let them be handled by their specific handlers
	if (c.req.path.startsWith("/.test/")) {
		return next();
	}

	// Strip basePath from the request path
	let strippedPath = c.req.path;
	if (runConfig.basePath && strippedPath.startsWith(runConfig.basePath)) {
		strippedPath = strippedPath.slice(runConfig.basePath.length);
		// Ensure the path starts with /
		if (!strippedPath.startsWith("/")) {
			strippedPath = "/" + strippedPath;
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
				runConfig,
				managerDriver,
				c,
				actorPathInfo,
			);
		}

		// Handle regular HTTP requests
		return await handleHttpGatewayPathBased(
			managerDriver,
			c,
			actorPathInfo,
		);
	}

	// Fallback to header-based routing
	// Check if this is a WebSocket upgrade request
	if (c.req.header("upgrade") === "websocket") {
		return await handleWebSocketGateway(
			runConfig,
			managerDriver,
			c,
			strippedPath,
		);
	}

	// Handle regular HTTP requests
	return await handleHttpGateway(managerDriver, c, next, strippedPath);
}

/**
 * Handle WebSocket requests using sec-websocket-protocol for routing
 */
async function handleWebSocketGateway(
	runConfig: RunnerConfig,
	managerDriver: ManagerDriver,
	c: HonoContext,
	strippedPath: string,
) {
	const upgradeWebSocket = runConfig.getUpgradeWebSocket?.();
	if (!upgradeWebSocket) {
		throw new WebSocketsNotEnabled();
	}

	// Parse configuration from Sec-WebSocket-Protocol header
	const protocols = c.req.header("sec-websocket-protocol");
	let target: string | undefined;
	let actorId: string | undefined;
	let encodingRaw: string | undefined;
	let connParamsRaw: string | undefined;

	if (protocols) {
		const protocolList = protocols.split(",").map((p) => p.trim());
		for (const protocol of protocolList) {
			if (protocol.startsWith(WS_PROTOCOL_TARGET)) {
				target = protocol.substring(WS_PROTOCOL_TARGET.length);
			} else if (protocol.startsWith(WS_PROTOCOL_ACTOR)) {
				actorId = decodeURIComponent(protocol.substring(WS_PROTOCOL_ACTOR.length));
			} else if (protocol.startsWith(WS_PROTOCOL_ENCODING)) {
				encodingRaw = protocol.substring(WS_PROTOCOL_ENCODING.length);
			} else if (protocol.startsWith(WS_PROTOCOL_CONN_PARAMS)) {
				connParamsRaw = decodeURIComponent(
					protocol.substring(WS_PROTOCOL_CONN_PARAMS.length),
				);
			}
		}
	}

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
		encoding: encodingRaw,
	});

	const encoding = encodingRaw || "json";
	const connParams = connParamsRaw ? JSON.parse(connParamsRaw) : undefined;

	// Include query string if present
	const pathWithQuery = c.req.url.includes("?")
		? strippedPath + c.req.url.substring(c.req.url.indexOf("?"))
		: strippedPath;

	return await managerDriver.proxyWebSocket(
		c,
		pathWithQuery,
		actorId,
		encoding as any, // Will be validated by driver
		connParams,
	);
}

/**
 * Handle HTTP requests using x-rivet headers for routing
 */
async function handleHttpGateway(
	managerDriver: ManagerDriver,
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

	return await managerDriver.proxyRequest(c, proxyRequest, actorId);
}

/**
 * Parse actor routing information from path
 * Matches patterns:
 * - /gateway/{actor_id}/{...path}
 * - /gateway/{actor_id}@{token}/{...path}
 */
export function parseActorPath(path: string): ActorPathInfo | null {
	// Find query string position (everything from ? onwards, but before fragment)
	const queryPos = path.indexOf("?");
	const fragmentPos = path.indexOf("#");

	// Extract query string (excluding fragment)
	let queryString = "";
	if (queryPos !== -1) {
		if (fragmentPos !== -1 && queryPos < fragmentPos) {
			queryString = path.slice(queryPos, fragmentPos);
		} else {
			queryString = path.slice(queryPos);
		}
	}

	// Extract base path (before query and fragment)
	let basePath = path;
	if (queryPos !== -1) {
		basePath = path.slice(0, queryPos);
	} else if (fragmentPos !== -1) {
		basePath = path.slice(0, fragmentPos);
	}

	// Check for double slashes (invalid path)
	if (basePath.includes("//")) {
		return null;
	}

	// Split the path into segments
	const segments = basePath.split("/").filter((s) => s.length > 0);

	// Check minimum required segments: gateway, {actor_id}
	if (segments.length < 2) {
		return null;
	}

	// Verify the first segment is "gateway"
	if (segments[0] !== "gateway") {
		return null;
	}

	// Extract actor_id segment (may contain @token)
	const actorSegment = segments[1];

	// Check for empty actor segment
	if (actorSegment.length === 0) {
		return null;
	}

	// Parse actor_id and optional token from the segment
	let actorId: string;
	let token: string | undefined;

	const atPos = actorSegment.indexOf("@");
	if (atPos !== -1) {
		// Pattern: /gateway/{actor_id}@{token}/{...path}
		const rawActorId = actorSegment.slice(0, atPos);
		const rawToken = actorSegment.slice(atPos + 1);

		// Check for empty actor_id or token
		if (rawActorId.length === 0 || rawToken.length === 0) {
			return null;
		}

		// URL-decode both actor_id and token
		try {
			actorId = decodeURIComponent(rawActorId);
			token = decodeURIComponent(rawToken);
		} catch (e) {
			// Invalid URL encoding
			return null;
		}
	} else {
		// Pattern: /gateway/{actor_id}/{...path}
		// URL-decode actor_id
		try {
			actorId = decodeURIComponent(actorSegment);
		} catch (e) {
			// Invalid URL encoding
			return null;
		}
		token = undefined;
	}

	// Calculate remaining path
	// The remaining path starts after /gateway/{actor_id[@token]}/
	let prefixLen = 0;
	for (let i = 0; i < 2; i++) {
		prefixLen += 1 + segments[i].length; // +1 for the slash
	}

	// Extract the remaining path preserving trailing slashes
	let remainingBase: string;
	if (prefixLen < basePath.length) {
		remainingBase = basePath.slice(prefixLen);
	} else {
		remainingBase = "/";
	}

	// Ensure remaining path starts with /
	let remainingPath: string;
	if (remainingBase.length === 0 || !remainingBase.startsWith("/")) {
		remainingPath = `/${remainingBase}${queryString}`;
	} else {
		remainingPath = `${remainingBase}${queryString}`;
	}

	return {
		actorId,
		token,
		remainingPath,
	};
}

/**
 * Creates a WebSocket proxy for test endpoints that forwards messages between server and client WebSockets
 */
export async function createTestWebSocketProxy(
	clientWsPromise: Promise<UniversalWebSocket>,
): Promise<UpgradeWebSocketArgs> {
	// Store a reference to the resolved WebSocket
	let clientWs: UniversalWebSocket | null = null;
	const {
		promise: serverWsPromise,
		resolve: serverWsResolve,
		reject: serverWsReject,
	} = promiseWithResolvers<WSContext>();
	try {
		// Resolve the client WebSocket promise
		logger().debug({ msg: "awaiting client websocket promise" });
		const ws = await clientWsPromise;
		clientWs = ws;
		logger().debug({
			msg: "client websocket promise resolved",
			constructor: ws?.constructor.name,
		});

		// Wait for ws to open
		await new Promise<void>((resolve, reject) => {
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
				serverWsReject();
			};

			ws.addEventListener("open", onOpen);

			ws.addEventListener("error", onError);

			ws.addEventListener("message", async (clientEvt: MessageEvent) => {
				const serverWs = await serverWsPromise;

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

				if (serverWs.readyState === 1) {
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
								serverWs.send(buffer as any);
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
						serverWs.send(clientEvt.data as any);
					}
				}
			});

			ws.addEventListener("close", async (clientEvt: any) => {
				const serverWs = await serverWsPromise;

				logger().debug({
					msg: `test websocket connection closed`,
				});

				if (serverWs.readyState !== 3) {
					// Not CLOSED
					serverWs.close(clientEvt.code, clientEvt.reason);
				}
			});

			ws.addEventListener("error", async () => {
				const serverWs = await serverWsPromise;

				logger().debug({
					msg: `test websocket connection error`,
				});

				if (serverWs.readyState !== 3) {
					// Not CLOSED
					serverWs.close(1011, "Error in client websocket");
				}
			});
		});
	} catch (error) {
		logger().error({
			msg: `failed to establish client websocket connection`,
			error,
		});
		return {
			onOpen: (_evt, serverWs) => {
				serverWs.close(1011, "Failed to establish connection");
			},
			onMessage: () => {},
			onError: () => {},
			onClose: () => {},
		};
	}

	// Create WebSocket proxy handlers to relay messages between client and server
	return {
		onOpen: (_evt: any, serverWs: WSContext) => {
			logger().debug({
				msg: `test websocket connection from client opened`,
			});

			// Check WebSocket type
			logger().debug({
				msg: "clientWs info",
				constructor: clientWs.constructor.name,
				hasAddEventListener:
					typeof clientWs.addEventListener === "function",
				readyState: clientWs.readyState,
			});

			serverWsResolve(serverWs);
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
			if (clientWs.readyState === 1) {
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
							clientWs.send(buffer);
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
					clientWs.send(evt.data);
				}
			}
		},
		onClose: (
			event: {
				wasClean: boolean;
				code: number;
				reason: string;
			},
			serverWs: WSContext,
		) => {
			logger().debug({
				msg: `server websocket closed`,
				wasClean: event.wasClean,
				code: event.code,
				reason: event.reason,
			});

			// HACK: Close socket in order to fix bug with Cloudflare leaving WS in closing state
			// https://github.com/cloudflare/workerd/issues/2569
			serverWs.close(1000, "hack_force_close");

			// Close the client websocket when the server websocket closes
			if (
				clientWs &&
				clientWs.readyState !== clientWs.CLOSED &&
				clientWs.readyState !== clientWs.CLOSING
			) {
				// Don't pass code/message since this may affect how close events are triggered
				clientWs.close(1000, event.reason);
			}
		},
		onError: (error: unknown) => {
			logger().error({
				msg: `error in server websocket`,
				error,
			});

			// Close the client websocket on error
			if (
				clientWs &&
				clientWs.readyState !== clientWs.CLOSED &&
				clientWs.readyState !== clientWs.CLOSING
			) {
				clientWs.close(1011, "Error in server websocket");
			}

			serverWsReject();
		},
	};
}
