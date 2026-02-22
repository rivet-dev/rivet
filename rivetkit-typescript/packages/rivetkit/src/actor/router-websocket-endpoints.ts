import type { WSContext } from "hono/ws";
import invariant from "invariant";
import type { AnyConn } from "@/actor/conn/mod";
import type { AnyActorInstance } from "@/actor/instance/mod";
import type { InputData } from "@/actor/protocol/serde";
import { type Encoding, EncodingSchema } from "@/actor/protocol/serde";
import {
	PATH_CONNECT,
	PATH_INSPECTOR_CONNECT,
	PATH_WEBSOCKET_BASE,
	PATH_WEBSOCKET_PREFIX,
	WS_PROTOCOL_CONN_PARAMS,
	WS_PROTOCOL_ENCODING,
	WS_PROTOCOL_INSPECTOR_TOKEN,
} from "@/common/actor-router-consts";
import { deconstructError } from "@/common/utils";
import type {
	RivetMessageEvent,
	UniversalWebSocket,
} from "@/common/websocket-interface";
import { handleWebSocketInspectorConnect } from "@/inspector/handler";
import type { RegistryConfig } from "@/registry/config";
import { promiseWithResolvers } from "@/utils";
import { timingSafeEqual } from "@/utils/crypto";
import type { ConnDriver } from "./conn/driver";
import { createRawWebSocketDriver } from "./conn/drivers/raw-websocket";
import { createWebSocketDriver } from "./conn/drivers/websocket";
import type { ActorDriver } from "./driver";
import { loggerWithoutContext } from "./log";
import { parseMessage } from "./protocol/old";
import { getRequestExposeInternalError } from "./router-endpoints";

// TODO: Merge with ConnectWebSocketOutput interface
export interface UpgradeWebSocketArgs {
	conn?: AnyConn;
	actor?: AnyActorInstance;
	onRestore?: (ws: WSContext) => void;
	onOpen: (event: any, ws: WSContext) => void;
	onMessage: (event: any, ws: WSContext) => void;
	onClose: (event: any, ws: WSContext) => void;
	onError: (error: any, ws: WSContext) => void;
}

interface WebSocketHandlerOpts {
	config: RegistryConfig;
	request: Request | undefined;
	encoding: Encoding;
	actor: AnyActorInstance;
	closePromiseResolvers: ReturnType<typeof promiseWithResolvers<void>>;
	conn: AnyConn;
	exposeInternalError: boolean;
}

/** Handler for a specific WebSocket route. Used in routeWebSocket. */
type WebSocketHandler = (
	opts: WebSocketHandlerOpts,
) => Promise<UpgradeWebSocketArgs>;

export async function routeWebSocket(
	request: Request | undefined,
	requestPath: string,
	requestHeaders: Record<string, string>,
	config: RegistryConfig,
	actorDriver: ActorDriver,
	actorId: string,
	encoding: Encoding,
	parameters: unknown,
	gatewayId: ArrayBuffer | undefined,
	requestId: ArrayBuffer | undefined,
	isHibernatable: boolean,
	isRestoringHibernatable: boolean,
): Promise<UpgradeWebSocketArgs> {
	const exposeInternalError = request
		? getRequestExposeInternalError(request)
		: false;

	let createdConn: AnyConn | undefined;
	try {
		const actor = await actorDriver.loadActor(actorId);

		actor.rLog.debug({
			msg: "new websocket connection",
			actorId,
			requestPath,
			isHibernatable,
		});

		// Promise used to wait for the websocket close in `disconnect`
		const closePromiseResolvers = promiseWithResolvers<void>((reason) => loggerWithoutContext().warn({ msg: "unhandled websocket close promise rejection", reason }));

		// Strip query parameters from requestPath for routing purposes.
		// This handles paths like "/websocket?query=value" which should route
		// to the raw websocket handler.
		const requestPathWithoutQuery = requestPath.split("?")[0];

		// Route WebSocket & create driver
		let handler: WebSocketHandler;
		let connDriver: ConnDriver;
		if (requestPathWithoutQuery === PATH_CONNECT) {
			const { driver, setWebSocket } = createWebSocketDriver(
				isHibernatable
					? { gatewayId: gatewayId!, requestId: requestId! }
					: undefined,
				encoding,
				closePromiseResolvers.promise,
				config,
			);
			handler = handleWebSocketConnect.bind(undefined, setWebSocket);
			connDriver = driver;
		} else if (
			requestPathWithoutQuery === PATH_WEBSOCKET_BASE ||
			requestPathWithoutQuery.startsWith(PATH_WEBSOCKET_PREFIX)
		) {
			const { driver, setWebSocket } = createRawWebSocketDriver(
				isHibernatable
					? { gatewayId: gatewayId!, requestId: requestId! }
					: undefined,
				closePromiseResolvers.promise,
			);
			handler = handleRawWebSocket.bind(undefined, setWebSocket);
			connDriver = driver;
		} else if (requestPathWithoutQuery === PATH_INSPECTOR_CONNECT) {
			if (!actor.inspectorToken) {
				throw "WebSocket Inspector Unauthorized: actor does not provide inspector access";
			}

			const inspectorToken = requestHeaders["sec-websocket-protocol"]
				.split(",")
				.map((p) => p.trim())
				.find((protocol) =>
					protocol.startsWith(WS_PROTOCOL_INSPECTOR_TOKEN),
				)
				// skip token prefix
				?.split(".")[1];

			if (
				!inspectorToken ||
				!timingSafeEqual(actor.inspectorToken, inspectorToken)
			) {
				throw "WebSocket Inspector Unauthorized: invalid token";
			}
			// This returns raw UpgradeWebSocketArgs instead of accepting a
			// Conn since this does not need a Conn
			return await handleWebSocketInspectorConnect({ actor });
		} else {
			throw `WebSocket Path Not Found: ${requestPath}`;
		}

		// Prepare connection
		const conn = await actor.connectionManager.prepareConn(
			connDriver,
			parameters,
			request,
			requestPath,
			requestHeaders,
			isHibernatable,
			isRestoringHibernatable,
		);
		createdConn = conn;

		// Create handler
		//
		// This must call actor.connectionManager.connectConn in onOpen.
		return await handler({
			config: config,
			request,
			encoding,
			actor,
			closePromiseResolvers,
			conn,
			exposeInternalError,
		});
	} catch (error) {
		const { group, code } = deconstructError(
			error,
			loggerWithoutContext(),
			{},
			exposeInternalError,
		);

		// Clean up connection
		if (createdConn) {
			createdConn.disconnect(`${group}.${code}`);
		}

		// Return handler that immediately closes with error
		// Note: createdConn should always exist here, but we use a type assertion for safety
		return {
			conn: createdConn!,
			onOpen: (_evt: any, ws: WSContext) => {
				ws.close(1011, `${group}.${code}`);
			},
			onMessage: (_evt: { data: any }, ws: WSContext) => {
				ws.close(1011, "actor.not_loaded");
			},
			onClose: (_event: any, _ws: WSContext) => { },
			onError: (_error: unknown) => { },
		};
	}
}

/**
 * Creates a WebSocket connection handler
 */
export async function handleWebSocketConnect(
	setWebSocket: (ws: WSContext) => void,
	{
		config: runConfig,
		encoding,
		actor,
		closePromiseResolvers,
		conn,
		exposeInternalError,
	}: WebSocketHandlerOpts,
): Promise<UpgradeWebSocketArgs> {
	// Process WS messages in order to avoid races between subscription updates
	// and subsequent action requests.
	let pendingMessage = Promise.resolve();

	return {
		conn,
		actor,
		onRestore: (ws: WSContext) => {
			setWebSocket(ws);
		},
		// NOTE: onOpen cannot be async since this messes up the open event listener order
		onOpen: (_evt: any, ws: WSContext) => {
			actor.rLog.debug("actor websocket open");

			setWebSocket(ws);

			// This will not be called by restoring hibernatable
			// connections. All restoration is done in prepareConn.
			actor.connectionManager.connectConn(conn);
		},
		onMessage: (evt: RivetMessageEvent, ws: WSContext) => {
			actor.rLog.debug({ msg: "received message" });
			const value = evt.data.valueOf() as InputData;
			pendingMessage = pendingMessage
				.then(async () => {
					const message = await parseMessage(value, {
						encoding: encoding,
						maxIncomingMessageSize: runConfig.maxIncomingMessageSize,
					});
					await actor.processMessage(message, conn);
				})
				.catch((error) => {
					const { group, code } = deconstructError(
						error,
						actor.rLog,
						{
							wsEvent: "message",
						},
						exposeInternalError,
					);
					ws.close(1011, `${group}.${code}`);
				});
		},
		onClose: (
			event: {
				wasClean: boolean;
				code: number;
				reason: string;
			},
			ws: WSContext,
		) => {
			closePromiseResolvers.resolve();

			if (event.wasClean) {
				actor.rLog.info({
					msg: "websocket closed",
					code: event.code,
					reason: event.reason,
					wasClean: event.wasClean,
				});
			} else {
				actor.rLog.warn({
					msg: "websocket closed",
					code: event.code,
					reason: event.reason,
					wasClean: event.wasClean,
				});
			}

			// HACK: Close socket in order to fix bug with Cloudflare leaving WS in closing state
			// https://github.com/cloudflare/workerd/issues/2569
			ws.close(1000, "hack_force_close");

			// Wait for actor.createConn to finish before removing the connection
			conn.disconnect(event?.reason);
		},
		onError: (_error: unknown) => {
			try {
				// Actors don't need to know about this, since it's abstracted away
				actor.rLog.warn({ msg: "websocket error" });
			} catch (error) {
				deconstructError(
					error,
					actor.rLog,
					{ wsEvent: "error" },
					exposeInternalError,
				);
			}
		},
	};
}

export async function handleRawWebSocket(
	setWebSocket: (ws: UniversalWebSocket) => void,
	{ request, actor, closePromiseResolvers, conn }: WebSocketHandlerOpts,
): Promise<UpgradeWebSocketArgs> {
	return {
		conn,
		actor,
		onRestore: (wsContext: WSContext) => {
			const ws = wsContext.raw as UniversalWebSocket;
			invariant(ws, "missing wsContext.raw");

			setWebSocket(ws);
		},
		// NOTE: onOpen cannot be async since this will cause the client's open
		// event to be called before this completes. Do all async work in
		// handleRawWebSocket root.
		onOpen: (_evt: any, wsContext: WSContext) => {
			const ws = wsContext.raw as UniversalWebSocket;
			invariant(ws, "missing wsContext.raw");

			setWebSocket(ws);

			// This will not be called by restoring hibernatable
			// connections. All restoration is done in prepareConn.
			actor.connectionManager.connectConn(conn);

			// Call the actor's onWebSocket handler with the adapted WebSocket
			//
			// NOTE: onWebSocket is called inside this function. Make sure
			// this is called synchronously within onOpen.
			actor.handleRawWebSocket(conn, ws, request);
		},
		// Raw websocket messages are handled directly by the actor's event
		// listeners on the WebSocket object, not through this callback
		onMessage: (_evt: any, _ws: any) => { },
		onClose: (evt: any, ws: any) => {
			// Resolve the close promise
			closePromiseResolvers.resolve();

			// Clean up the connection
			conn.disconnect(evt?.reason);
		},
		onError: (error: any, ws: any) => { },
	};
}

export interface WebSocketCustomProtocols {
	encoding: Encoding;
	connParams: unknown;
}

/**
 * Parse encoding and connection parameters from WebSocket Sec-WebSocket-Protocol header
 */
export function parseWebSocketProtocols(
	protocols: string | null | undefined,
): WebSocketCustomProtocols {
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

	// Default to "json" encoding for raw WebSocket connections without subprotocols
	const encoding = EncodingSchema.parse(encodingRaw ?? "json");
	const connParams = connParamsRaw ? JSON.parse(connParamsRaw) : undefined;

	return { encoding, connParams };
}

/**
 * Truncase the PATH_WEBSOCKET_PREFIX path prefix in order to pass a clean
 * path to the onWebSocket handler.
 *
 * Example:
 * - `/websocket/foo` -> `/foo`
 * - `/websocket` -> `/`
 */
export function truncateRawWebSocketPathPrefix(path: string): string {
	// Extract the path after prefix and preserve query parameters
	// Use URL API for cleaner parsing
	const url = new URL(path, "http://actor");
	const pathname = url.pathname.replace(/^\/websocket\/?/, "") || "/";
	const normalizedPath =
		(pathname.startsWith("/") ? pathname : `/${pathname}`) + url.search;

	return normalizedPath;
}
