import * as cbor from "cbor-x";
import type { Context as HonoContext, HonoRequest } from "hono";
import type { WSContext } from "hono/ws";
import type { AnyConn } from "@/actor/conn/mod";
import { ActionContext } from "@/actor/contexts/action";
import * as errors from "@/actor/errors";
import type { AnyActorInstance } from "@/actor/instance/mod";
import type { InputData } from "@/actor/protocol/serde";
import { type Encoding, EncodingSchema } from "@/actor/protocol/serde";
import {
	HEADER_ACTOR_QUERY,
	HEADER_CONN_PARAMS,
	HEADER_ENCODING,
	WS_PROTOCOL_CONN_PARAMS,
	WS_PROTOCOL_ENCODING,
} from "@/common/actor-router-consts";
import type { UpgradeWebSocketArgs } from "@/common/inline-websocket-adapter2";
import { deconstructError, stringifyError } from "@/common/utils";
import type { UniversalWebSocket } from "@/common/websocket-interface";
import { HonoWebSocketAdapter } from "@/manager/hono-websocket-adapter";
import type { RunnerConfig } from "@/registry/run-config";
import type * as protocol from "@/schemas/client-protocol/mod";
import {
	HTTP_ACTION_REQUEST_VERSIONED,
	HTTP_ACTION_RESPONSE_VERSIONED,
} from "@/schemas/client-protocol/versioned";
import {
	type HttpActionRequest as HttpActionRequestJson,
	HttpActionRequestSchema,
	type HttpActionResponse as HttpActionResponseJson,
	HttpActionResponseSchema,
} from "@/schemas/client-protocol-zod/mod";
import {
	contentTypeForEncoding,
	deserializeWithEncoding,
	serializeWithEncoding,
} from "@/serde";
import {
	arrayBuffersEqual,
	bufferToArrayBuffer,
	idToStr,
	promiseWithResolvers,
} from "@/utils";
import { createHttpSocket } from "./conn/drivers/http";
import { createRawRequestSocket } from "./conn/drivers/raw-request";
import { createRawWebSocketSocket } from "./conn/drivers/raw-websocket";
import { createWebSocketSocket } from "./conn/drivers/websocket";
import type { ActorDriver } from "./driver";
import { loggerWithoutContext } from "./log";
import { parseMessage } from "./protocol/old";

export interface ConnectWebSocketOpts {
	req?: HonoRequest;
	encoding: Encoding;
	actorId: string;
	params: unknown;
}

export interface ConnectWebSocketOutput {
	onOpen: (ws: WSContext) => void;
	onMessage: (message: protocol.ToServer) => void;
	onClose: () => void;
}

export interface ActionOpts {
	req?: HonoRequest;
	params: unknown;
	actionName: string;
	actionArgs: unknown[];
	actorId: string;
}

export interface ActionOutput {
	output: unknown;
}

export interface ConnsMessageOpts {
	req?: HonoRequest;
	connId: string;
	message: protocol.ToServer;
	actorId: string;
}

export interface FetchOpts {
	request: Request;
	actorId: string;
}

export interface WebSocketOpts {
	request: Request;
	websocket: UniversalWebSocket;
	actorId: string;
}

/**
 * Creates a WebSocket connection handler
 */
export async function handleWebSocketConnect(
	req: Request | undefined,
	runConfig: RunnerConfig,
	actorDriver: ActorDriver,
	actorId: string,
	encoding: Encoding,
	parameters: unknown,
	requestId: string,
	requestIdBuf: ArrayBuffer | undefined,
): Promise<UpgradeWebSocketArgs> {
	const exposeInternalError = req
		? getRequestExposeInternalError(req)
		: false;

	let createdConn: AnyConn | undefined;
	try {
		const actor = await actorDriver.loadActor(actorId);

		// Promise used to wait for the websocket close in `disconnect`
		const closePromiseResolvers = promiseWithResolvers<void>();

		actor.rLog.debug({
			msg: "new websocket connection",
			actorId,
		});

		// Check if this is a hibernatable websocket
		const isHibernatable =
			!!requestIdBuf &&
			actor.persist.hibernatableConns.findIndex((conn) =>
				arrayBuffersEqual(conn.hibernatableRequestId, requestIdBuf),
			) !== -1;

		const { driver, setWebSocket } = createWebSocketSocket(
			requestId,
			requestIdBuf,
			isHibernatable,
			encoding,
			closePromiseResolvers.promise,
		);
		const conn = await actor.connectionManager.prepareConn(
			driver,
			parameters,
			req,
		);
		createdConn = conn;

		return {
			// NOTE: onOpen cannot be async since this messes up the open event listener order
			onOpen: (_evt: any, ws: WSContext) => {
				actor.rLog.debug("actor websocket open");

				setWebSocket(ws);

				actor.connectionManager.connectConn(conn);
			},
			onMessage: (evt: { data: any }, ws: WSContext) => {
				// Handle message asynchronously
				actor.rLog.debug({ msg: "received message" });

				const value = evt.data.valueOf() as InputData;
				parseMessage(value, {
					encoding: encoding,
					maxIncomingMessageSize: runConfig.maxIncomingMessageSize,
				})
					.then((message) => {
						actor.processMessage(message, conn).catch((error) => {
							const { code } = deconstructError(
								error,
								actor.rLog,
								{
									wsEvent: "message",
								},
								exposeInternalError,
							);
							ws.close(1011, code);
						});
					})
					.catch((error) => {
						const { code } = deconstructError(
							error,
							actor.rLog,
							{
								wsEvent: "message",
							},
							exposeInternalError,
						);
						ws.close(1011, code);
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
				if (createdConn) {
					createdConn.disconnect(event?.reason);
				}
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
		return {
			onOpen: (_evt: any, ws: WSContext) => {
				ws.close(1011, code);
			},
			onMessage: (_evt: { data: any }, ws: WSContext) => {
				ws.close(1011, "Actor not loaded");
			},
			onClose: (_event: any, _ws: WSContext) => {},
			onError: (_error: unknown) => {},
		};
	}
}

/**
 * Creates an action handler
 */
export async function handleAction(
	c: HonoContext,
	_runConfig: RunnerConfig,
	actorDriver: ActorDriver,
	actionName: string,
	actorId: string,
) {
	const encoding = getRequestEncoding(c.req);
	const parameters = getRequestConnParams(c.req);

	// Validate incoming request
	const arrayBuffer = await c.req.arrayBuffer();
	const request = deserializeWithEncoding(
		encoding,
		new Uint8Array(arrayBuffer),
		HTTP_ACTION_REQUEST_VERSIONED,
		HttpActionRequestSchema,
		// JSON: args is already the decoded value (raw object/array)
		(json: HttpActionRequestJson) => json.args,
		// BARE/CBOR: args is ArrayBuffer that needs CBOR-decoding
		(bare: protocol.HttpActionRequest) =>
			cbor.decode(new Uint8Array(bare.args)),
	);
	const actionArgs = request;

	// Invoke the action
	let actor: AnyActorInstance | undefined;
	let conn: AnyConn | undefined;
	let output: unknown | undefined;
	try {
		actor = await actorDriver.loadActor(actorId);

		actor.rLog.debug({ msg: "handling action", actionName, encoding });

		// Create conn
		conn = await actor.connectionManager.prepareAndConnectConn(
			createHttpSocket(),
			parameters,
			c.req.raw,
		);

		// Call action
		const ctx = new ActionContext(actor, conn!);
		output = await actor.executeAction(ctx, actionName, actionArgs);
	} finally {
		if (conn) {
			conn.disconnect();
		}
	}

	// Send response
	const serialized = serializeWithEncoding(
		encoding,
		output,
		HTTP_ACTION_RESPONSE_VERSIONED,
		HttpActionResponseSchema,
		// JSON: output is the raw value (will be serialized by jsonStringifyCompat)
		(value): HttpActionResponseJson => ({ output: value }),
		// BARE/CBOR: output needs to be CBOR-encoded to ArrayBuffer
		(value): protocol.HttpActionResponse => ({
			output: bufferToArrayBuffer(cbor.encode(value)),
		}),
	);

	// TODO: Remvoe any, Hono is being a dumbass
	return c.body(serialized as Uint8Array as any, 200, {
		"Content-Type": contentTypeForEncoding(encoding),
	});
}

export async function handleRawRequest(
	req: Request,
	actorDriver: ActorDriver,
	actorId: string,
): Promise<Response> {
	const actor = await actorDriver.loadActor(actorId);

	// Track connection outside of scope for cleanup
	let createdConn: AnyConn | undefined;

	try {
		const conn = await actor.connectionManager.prepareAndConnectConn(
			createRawRequestSocket(),
			{},
			req,
		);

		createdConn = conn;

		return await actor.handleRawRequest(conn, req);
	} finally {
		// Clean up the connection after the request completes
		if (createdConn) {
			createdConn.disconnect();
		}
	}
}

export async function handleRawWebSocket(
	req: Request | undefined,
	path: string,
	actorDriver: ActorDriver,
	actorId: string,
	requestIdBuf: ArrayBuffer | undefined,
): Promise<UpgradeWebSocketArgs> {
	const exposeInternalError = req
		? getRequestExposeInternalError(req)
		: false;

	let createdConn: AnyConn | undefined;
	try {
		const actor = await actorDriver.loadActor(actorId);

		// Promise used to wait for the websocket close in `disconnect`
		const closePromiseResolvers = promiseWithResolvers<void>();

		// Extract rivetRequestId provided by engine runner
		const isHibernatable =
			!!requestIdBuf &&
			actor.persist.hibernatableConns.findIndex((conn) =>
				arrayBuffersEqual(conn.hibernatableRequestId, requestIdBuf),
			) !== -1;

		const newPath = truncateRawWebSocketPathPrefix(path);
		let newRequest: Request;
		if (req) {
			newRequest = new Request(`http://actor${newPath}`, req);
		} else {
			newRequest = new Request(`http://actor${newPath}`, {
				method: "GET",
			});
		}

		actor.rLog.debug({
			msg: "rewriting websocket url",
			fromPath: path,
			toUrl: newRequest.url,
		});
		// Create connection using actor.createConn - this handles deduplication for hibernatable connections
		const requestIdStr = requestIdBuf
			? idToStr(requestIdBuf)
			: crypto.randomUUID();
		const { driver, setWebSocket } = createRawWebSocketSocket(
			requestIdStr,
			requestIdBuf,
			isHibernatable,
			closePromiseResolvers.promise,
		);
		const conn = await actor.connectionManager.prepareAndConnectConn(
			driver,
			{},
			newRequest,
		);
		createdConn = conn;

		// Return WebSocket event handlers
		return {
			// NOTE: onOpen cannot be async since this will cause the client's open
			// event to be called before this completes. Do all async work in
			// handleRawWebSocket root.
			onOpen: (_evt: any, ws: any) => {
				// Wrap the Hono WebSocket in our adapter
				const adapter = new HonoWebSocketAdapter(
					ws,
					requestIdBuf,
					isHibernatable,
				);

				// Store adapter reference on the WebSocket for event handlers
				(ws as any).__adapter = adapter;

				setWebSocket(adapter);

				// Call the actor's onWebSocket handler with the adapted WebSocket
				//
				// NOTE: onWebSocket is called inside this function. Make sure
				// this is called synchronously within onOpen.
				actor.handleRawWebSocket(conn, adapter, newRequest);
			},
			onMessage: (event: any, ws: any) => {
				// Find the adapter for this WebSocket
				const adapter = (ws as any).__adapter;
				if (adapter) {
					adapter._handleMessage(event);
				}
			},
			onClose: (evt: any, ws: any) => {
				// Find the adapter for this WebSocket
				const adapter = (ws as any).__adapter;
				if (adapter) {
					adapter._handleClose(evt?.code || 1006, evt?.reason || "");
				}

				// Resolve the close promise
				closePromiseResolvers.resolve();

				// Clean up the connection
				if (createdConn) {
					createdConn.disconnect(evt?.reason);
				}
			},
			onError: (error: any, ws: any) => {
				// Find the adapter for this WebSocket
				const adapter = (ws as any).__adapter;
				if (adapter) {
					adapter._handleError(error);
				}
			},
		};
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
		return {
			onOpen: (_evt: any, ws: WSContext) => {
				ws.close(1011, code);
			},
			onMessage: (_evt: { data: any }, ws: WSContext) => {
				ws.close(1011, "Actor not loaded");
			},
			onClose: (_event: any, _ws: WSContext) => {},
			onError: (_error: unknown) => {},
		};
	}
}

// Helper to get the connection encoding from a request
//
// Defaults to JSON if not provided so we can support vanilla curl requests easily.
export function getRequestEncoding(req: HonoRequest): Encoding {
	const encodingParam = req.header(HEADER_ENCODING);
	if (!encodingParam) {
		return "json";
	}

	const result = EncodingSchema.safeParse(encodingParam);
	if (!result.success) {
		throw new errors.InvalidEncoding(encodingParam as string);
	}

	return result.data;
}

export function getRequestExposeInternalError(_req: Request): boolean {
	// Unipmlemented
	return false;
}

export function getRequestQuery(c: HonoContext): unknown {
	// Get query parameters for actor lookup
	const queryParam = c.req.header(HEADER_ACTOR_QUERY);
	if (!queryParam) {
		loggerWithoutContext().error({ msg: "missing query parameter" });
		throw new errors.InvalidRequest("missing query");
	}

	// Parse the query JSON and validate with schema
	try {
		const parsed = JSON.parse(queryParam);
		return parsed;
	} catch (error) {
		loggerWithoutContext().error({ msg: "invalid query json", error });
		throw new errors.InvalidQueryJSON(error);
	}
}

// Helper to get connection parameters for the request
export function getRequestConnParams(req: HonoRequest): unknown {
	const paramsParam = req.header(HEADER_CONN_PARAMS);
	if (!paramsParam) {
		return null;
	}

	try {
		return JSON.parse(paramsParam);
	} catch (err) {
		throw new errors.InvalidParams(
			`Invalid params JSON: ${stringifyError(err)}`,
		);
	}
}

/**
 * Parse encoding and connection parameters from WebSocket Sec-WebSocket-Protocol header
 */
export function parseWebSocketProtocols(protocols: string | null | undefined): {
	encoding: Encoding;
	connParams: unknown;
} {
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

	const encoding = EncodingSchema.parse(encodingRaw);
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
		(pathname.startsWith("/") ? pathname : "/" + pathname) + url.search;

	return normalizedPath;
}
