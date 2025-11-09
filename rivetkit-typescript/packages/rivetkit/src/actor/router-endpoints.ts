import * as cbor from "cbor-x";
import type { Context as HonoContext, HonoRequest } from "hono";
import type { WSContext } from "hono/ws";
import type { AnyConn } from "@/actor/conn/mod";
import { ActionContext } from "@/actor/contexts/action";
import * as errors from "@/actor/errors";
import {
	ACTOR_INSTANCE_PERSIST_SYMBOL,
	type AnyActorInstance,
} from "@/actor/instance/mod";
import type { InputData } from "@/actor/protocol/serde";
import { type Encoding, EncodingSchema } from "@/actor/protocol/serde";
import {
	HEADER_ACTOR_QUERY,
	HEADER_CONN_PARAMS,
	HEADER_ENCODING,
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
import { createRawHttpSocket } from "./conn/drivers/raw-http";
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

	// Setup promise for the init handlers since all other behavior depends on this
	const {
		promise: handlersPromise,
		resolve: handlersResolve,
		reject: handlersReject,
	} = promiseWithResolvers<{
		conn: AnyConn;
		actor: AnyActorInstance;
		connId: string;
	}>();

	// Pre-load the actor to catch errors early
	let actor: AnyActorInstance;
	try {
		actor = await actorDriver.loadActor(actorId);
	} catch (error) {
		// Return handler that immediately closes with error
		return {
			onOpen: (_evt: any, ws: WSContext) => {
				const { code } = deconstructError(
					error,
					loggerWithoutContext(),
					{
						wsEvent: "open",
					},
					exposeInternalError,
				);
				ws.close(1011, code);
			},
			onMessage: (_evt: { data: any }, ws: WSContext) => {
				ws.close(1011, "Actor not loaded");
			},
			onClose: (_event: any, _ws: WSContext) => {},
			onError: (_error: unknown) => {},
		};
	}

	// Promise used to wait for the websocket close in `disconnect`
	const closePromiseResolvers = promiseWithResolvers<void>();

	// Track connection outside of scope for cleanup
	let createdConn: AnyConn | undefined;

	return {
		onOpen: (_evt: any, ws: WSContext) => {
			actor.rLog.debug("actor websocket open");

			// Run async operations in background
			(async () => {
				try {
					let conn: AnyConn;

					actor.rLog.debug({
						msg: "new websocket connection",
						actorId,
					});

					// Check if this is a hibernatable websocket
					const isHibernatable =
						!!requestIdBuf &&
						actor[
							ACTOR_INSTANCE_PERSIST_SYMBOL
						].hibernatableConns.findIndex((conn) =>
							arrayBuffersEqual(
								conn.hibernatableRequestId,
								requestIdBuf,
							),
						) !== -1;

					conn = await actor.createConn(
						createWebSocketSocket(
							requestId,
							requestIdBuf,
							isHibernatable,
							encoding,
							ws,
							closePromiseResolvers.promise,
						),
						parameters,
						req,
					);

					// Store connection so we can clean on close
					createdConn = conn;

					// Unblock other handlers
					handlersResolve({ conn, actor, connId: conn.id });
				} catch (error) {
					handlersReject(error);

					const { code } = deconstructError(
						error,
						actor.rLog,
						{
							wsEvent: "open",
						},
						exposeInternalError,
					);
					ws.close(1011, code);
				}
			})();
		},
		onMessage: (evt: { data: any }, ws: WSContext) => {
			// Handle message asynchronously
			handlersPromise
				.then(({ conn, actor }) => {
					actor.rLog.debug({ msg: "received message" });

					const value = evt.data.valueOf() as InputData;
					parseMessage(value, {
						encoding: encoding,
						maxIncomingMessageSize:
							runConfig.maxIncomingMessageSize,
					})
						.then((message) => {
							actor
								.processMessage(message, conn)
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
			handlersReject(`WebSocket closed (${event.code}): ${event.reason}`);

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
			handlersPromise.finally(() => {
				if (createdConn) {
					const wasClean = event.wasClean || event.code === 1000;
					actor.connDisconnected(createdConn, wasClean);
				}
			});
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
	);
	const actionArgs = cbor.decode(new Uint8Array(request.args));

	// Invoke the action
	let actor: AnyActorInstance | undefined;
	let conn: AnyConn | undefined;
	let output: unknown | undefined;
	try {
		actor = await actorDriver.loadActor(actorId);

		actor.rLog.debug({ msg: "handling action", actionName, encoding });

		// Create conn
		conn = await actor.createConn(
			createHttpSocket(),
			parameters,
			c.req.raw,
		);

		// Call action
		const ctx = new ActionContext(actor.actorContext!, conn!);
		output = await actor.executeAction(ctx, actionName, actionArgs);
	} finally {
		if (conn) {
			// HTTP connections don't have persistent sockets, so no socket ID needed
			actor?.connDisconnected(conn, true);
		}
	}

	// Send response
	const responseData: protocol.HttpActionResponse = {
		output: bufferToArrayBuffer(cbor.encode(output)),
	};
	const serialized = serializeWithEncoding(
		encoding,
		responseData,
		HTTP_ACTION_RESPONSE_VERSIONED,
	);

	// TODO: Remvoe any, Hono is being a dumbass
	return c.body(serialized as Uint8Array as any, 200, {
		"Content-Type": contentTypeForEncoding(encoding),
	});
}

export async function handleRawWebSocketHandler(
	req: Request | undefined,
	path: string,
	actorDriver: ActorDriver,
	actorId: string,
	requestIdBuf: ArrayBuffer | undefined,
): Promise<UpgradeWebSocketArgs> {
	const actor = await actorDriver.loadActor(actorId);

	// Promise used to wait for the websocket close in `disconnect`
	const closePromiseResolvers = promiseWithResolvers<void>();

	// Track connection outside of scope for cleanup
	let createdConn: AnyConn | undefined;

	// Return WebSocket event handlers
	return {
		onOpen: async (evt: any, ws: any) => {
			// Extract rivetRequestId provided by engine runner
			const isHibernatable =
				!!requestIdBuf &&
				actor[
					ACTOR_INSTANCE_PERSIST_SYMBOL
				].hibernatableConns.findIndex((conn) =>
					arrayBuffersEqual(conn.hibernatableRequestId, requestIdBuf),
				) !== -1;

			// Wrap the Hono WebSocket in our adapter
			const adapter = new HonoWebSocketAdapter(
				ws,
				requestIdBuf,
				isHibernatable,
			);

			// Store adapter reference on the WebSocket for event handlers
			(ws as any).__adapter = adapter;

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

			try {
				// Create connection using actor.createConn - this handles deduplication for hibernatable connections
				const requestIdStr = requestIdBuf
					? idToStr(requestIdBuf)
					: crypto.randomUUID();
				const conn = await actor.createConn(
					createRawWebSocketSocket(
						requestIdStr,
						requestIdBuf,
						isHibernatable,
						adapter,
						closePromiseResolvers.promise,
					),
					{}, // No parameters for raw WebSocket
					newRequest,
				);

				createdConn = conn;

				// Call the actor's onWebSocket handler with the adapted WebSocket
				actor.handleRawWebSocket(adapter, {
					request: newRequest,
				});
			} catch (error) {
				actor.rLog.error({
					msg: "failed to create raw WebSocket connection",
					error: String(error),
				});
				ws.close(1011, "Failed to create connection");
			}
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
				const wasClean = evt?.wasClean || evt?.code === 1000;
				actor.connDisconnected(createdConn, wasClean);
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
}

// Helper to get the connection encoding from a request
export function getRequestEncoding(req: HonoRequest): Encoding {
	const encodingParam = req.header(HEADER_ENCODING);
	if (!encodingParam) {
		throw new errors.InvalidEncoding("undefined");
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

export async function handleRawHttpHandler(
	req: Request,
	actorDriver: ActorDriver,
	actorId: string,
): Promise<Response> {
	const actor = await actorDriver.loadActor(actorId);

	// Track connection outside of scope for cleanup
	let createdConn: AnyConn | undefined;

	try {
		const conn = await actor.createConn(createRawHttpSocket(), {}, req);

		createdConn = conn;

		return await actor.handleRawRequest(req, {});
	} finally {
		// Clean up the connection after the request completes
		if (createdConn) {
			actor.connDisconnected(createdConn, true);
		}
	}
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
