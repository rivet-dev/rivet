// KV Channel WebSocket handler for the local dev manager.
//
// Serves the /kv/connect endpoint that the native SQLite addon
// (rivetkit-typescript/packages/sqlite-native/) connects to for
// KV-backed database I/O. See docs-internal/engine/NATIVE_SQLITE_DATA_CHANNEL.md
// for the full specification.

import type { WSContext } from "hono/ws";
import {
	PROTOCOL_VERSION,
	type ToServer,
	type ToClient,
	type RequestData,
	type ResponseData,
	type ToServerRequest,
	decodeToServer,
	encodeToClient,
} from "@rivetkit/engine-kv-channel-protocol";
import type { ManagerDriver } from "./driver";
import { logger } from "./log";

// Ping every 3 seconds, close if no pong within 15 seconds.
// Matches runner protocol defaults (runner_update_ping_interval_ms=3000,
// runner_ping_timeout_ms=15000 in engine/packages/config/src/config/pegboard.rs).
const PING_INTERVAL_MS = 3_000;
const PONG_TIMEOUT_MS = 15_000;

/** Per-connection state for the KV channel WebSocket. */
interface KvChannelConnection {
	/** Actor IDs locked by this connection. */
	openActors: Set<string>;

	/** Timer for sending pings. */
	pingInterval: ReturnType<typeof setInterval> | null;

	/** Timer for detecting pong timeout. */
	pongTimeout: ReturnType<typeof setTimeout> | null;

	/** Timestamp of the last pong received. */
	lastPongTs: number;

	/** Whether the connection has been closed. */
	closed: boolean;

	/** Reference to the WebSocket context for sending messages. */
	ws: WSContext | null;
}

/** Global lock table: actorId -> connectionId. Shared across all connections. */
const actorLocks = new Map<string, KvChannelConnection>();

function makeErrorResponse(
	requestId: number,
	code: string,
	message: string,
): ToClient {
	return {
		tag: "ToClientResponse",
		val: {
			requestId,
			data: {
				tag: "ErrorResponse",
				val: { code, message },
			},
		},
	};
}

function makeResponse(requestId: number, data: ResponseData): ToClient {
	return {
		tag: "ToClientResponse",
		val: { requestId, data },
	};
}

function sendMessage(ws: WSContext, msg: ToClient): void {
	const bytes = encodeToClient(msg);
	// Copy to a fresh ArrayBuffer to satisfy WSContext.send() parameter type.
	const copy = new ArrayBuffer(bytes.byteLength);
	new Uint8Array(copy).set(bytes);
	ws.send(copy);
}

function startPingPong(conn: KvChannelConnection): void {
	conn.lastPongTs = Date.now();

	conn.pingInterval = setInterval(() => {
		if (conn.closed || !conn.ws) return;

		const ts = BigInt(Date.now());
		sendMessage(conn.ws, {
			tag: "ToClientPing",
			val: { ts },
		});

		// Check if the last pong was too long ago.
		if (Date.now() - conn.lastPongTs > PONG_TIMEOUT_MS) {
			logger().warn({
				msg: "kv channel pong timeout, closing connection",
			});
			cleanupConnection(conn);
			conn.ws.close(1000, "pong timeout");
		}
	}, PING_INTERVAL_MS);
}

function cleanupConnection(conn: KvChannelConnection): void {
	conn.closed = true;

	if (conn.pingInterval) {
		clearInterval(conn.pingInterval);
		conn.pingInterval = null;
	}
	if (conn.pongTimeout) {
		clearTimeout(conn.pongTimeout);
		conn.pongTimeout = null;
	}

	// Release all actor locks held by this connection.
	for (const actorId of conn.openActors) {
		if (actorLocks.get(actorId) === conn) {
			actorLocks.delete(actorId);
		}
	}
	conn.openActors.clear();
}

async function handleRequest(
	conn: KvChannelConnection,
	ws: WSContext,
	managerDriver: ManagerDriver,
	request: ToServerRequest,
): Promise<void> {
	const { requestId, actorId, data } = request;

	try {
		const responseData = await processRequestData(
			conn,
			managerDriver,
			actorId,
			data,
		);
		sendMessage(ws, makeResponse(requestId, responseData));
	} catch (err: unknown) {
		const message =
			err instanceof Error ? err.message : "unexpected error";
		logger().error({
			msg: "kv channel request error",
			requestId,
			actorId,
			error: message,
		});
		sendMessage(
			ws,
			makeErrorResponse(requestId, "internal_error", message),
		);
	}
}

async function processRequestData(
	conn: KvChannelConnection,
	managerDriver: ManagerDriver,
	actorId: string,
	data: RequestData,
): Promise<ResponseData> {
	switch (data.tag) {
		case "ActorOpenRequest":
			return handleActorOpen(conn, actorId);

		case "ActorCloseRequest":
			return handleActorClose(conn, actorId);

		case "KvGetRequest":
		case "KvPutRequest":
		case "KvDeleteRequest":
		case "KvDeleteRangeRequest": {
			// All KV operations require the actor to be open on this connection.
			const lockHolder = actorLocks.get(actorId);
			if (!lockHolder || lockHolder !== conn) {
				if (lockHolder && lockHolder !== conn) {
					return {
						tag: "ErrorResponse",
						val: {
							code: "actor_locked",
							message: `actor ${actorId} is locked by another connection`,
						},
					};
				}
				return {
					tag: "ErrorResponse",
					val: {
						code: "actor_not_open",
						message: `actor ${actorId} is not open on this connection`,
					},
				};
			}
			return await handleKvOperation(managerDriver, actorId, data);
		}
	}
}

function handleActorOpen(
	conn: KvChannelConnection,
	actorId: string,
): ResponseData {
	const existingLock = actorLocks.get(actorId);
	if (existingLock && existingLock !== conn) {
		// Unconditionally evict the old connection's lock. The old connection
		// is either dead (network issue) or stale (same process reconnecting).
		// Remove the actor from the old connection's openActors so its next KV
		// request fails the fast-path check immediately with actor_not_open.
		existingLock.openActors.delete(actorId);
		logger().info({
			msg: "kv channel evicting actor lock from old connection",
			actorId,
		});
	}

	actorLocks.set(actorId, conn);
	conn.openActors.add(actorId);

	return { tag: "ActorOpenResponse", val: null };
}

function handleActorClose(
	conn: KvChannelConnection,
	actorId: string,
): ResponseData {
	if (actorLocks.get(actorId) === conn) {
		actorLocks.delete(actorId);
	}
	conn.openActors.delete(actorId);

	return { tag: "ActorCloseResponse", val: null };
}

type KvRequestData = Extract<
	RequestData,
	| { readonly tag: "KvGetRequest" }
	| { readonly tag: "KvPutRequest" }
	| { readonly tag: "KvDeleteRequest" }
	| { readonly tag: "KvDeleteRangeRequest" }
>;

async function handleKvOperation(
	managerDriver: ManagerDriver,
	actorId: string,
	data: KvRequestData,
): Promise<ResponseData> {
	switch (data.tag) {
		case "KvGetRequest": {
			const keys = data.val.keys.map(
				(k) => new Uint8Array(k),
			);

			// Validate key count.
			if (keys.length > 128) {
				return {
					tag: "ErrorResponse",
					val: {
						code: "batch_too_large",
						message: "a maximum of 128 keys is allowed",
					},
				};
			}

			// Validate individual key sizes.
			for (const key of keys) {
				if (key.byteLength + 2 > 2048) {
					return {
						tag: "ErrorResponse",
						val: {
							code: "key_too_large",
							message: "key is too long (max 2048 bytes)",
						},
					};
				}
			}

			const results = await managerDriver.kvBatchGet(actorId, keys);

			// Return only found keys and values.
			const foundKeys: ArrayBuffer[] = [];
			const foundValues: ArrayBuffer[] = [];
			for (let i = 0; i < keys.length; i++) {
				const val = results[i];
				if (val !== null) {
					foundKeys.push(new Uint8Array(keys[i]).buffer as ArrayBuffer);
					foundValues.push(new Uint8Array(val).buffer as ArrayBuffer);
				}
			}

			return {
				tag: "KvGetResponse",
				val: { keys: foundKeys, values: foundValues },
			};
		}

		case "KvPutRequest": {
			const keys = data.val.keys.map(
				(k) => new Uint8Array(k),
			);
			const values = data.val.values.map(
				(v) => new Uint8Array(v),
			);

			if (keys.length !== values.length) {
				return {
					tag: "ErrorResponse",
					val: {
						code: "keys_values_length_mismatch",
						message:
							"keys and values arrays must have the same length",
					},
				};
			}

			if (keys.length > 128) {
				return {
					tag: "ErrorResponse",
					val: {
						code: "batch_too_large",
						message:
							"a maximum of 128 key-value entries is allowed",
					},
				};
			}

			// Validate sizes.
			let payloadSize = 0;
			for (let i = 0; i < keys.length; i++) {
				if (keys[i].byteLength + 2 > 2048) {
					return {
						tag: "ErrorResponse",
						val: {
							code: "key_too_large",
							message: "key is too long (max 2048 bytes)",
						},
					};
				}
				if (values[i].byteLength > 128 * 1024) {
					return {
						tag: "ErrorResponse",
						val: {
							code: "value_too_large",
							message: "value is too large (max 128 KiB)",
						},
					};
				}
				payloadSize +=
					keys[i].byteLength + 2 + values[i].byteLength;
			}

			if (payloadSize > 976 * 1024) {
				return {
					tag: "ErrorResponse",
					val: {
						code: "payload_too_large",
						message:
							"total payload is too large (max 976 KiB)",
					},
				};
			}

			const entries: [Uint8Array, Uint8Array][] = keys.map(
				(k, i) => [k, values[i]],
			);

			try {
				await managerDriver.kvBatchPut(actorId, entries);
			} catch (err: unknown) {
				const message =
					err instanceof Error ? err.message : String(err);
				if (message.includes("not enough space")) {
					return {
						tag: "ErrorResponse",
						val: {
							code: "storage_quota_exceeded",
							message,
						},
					};
				}
				throw err;
			}

			return { tag: "KvPutResponse", val: null };
		}

		case "KvDeleteRequest": {
			const keys = data.val.keys.map(
				(k) => new Uint8Array(k),
			);

			if (keys.length > 128) {
				return {
					tag: "ErrorResponse",
					val: {
						code: "batch_too_large",
						message: "a maximum of 128 keys is allowed",
					},
				};
			}

			for (const key of keys) {
				if (key.byteLength + 2 > 2048) {
					return {
						tag: "ErrorResponse",
						val: {
							code: "key_too_large",
							message: "key is too long (max 2048 bytes)",
						},
					};
				}
			}

			await managerDriver.kvBatchDelete(actorId, keys);

			return { tag: "KvDeleteResponse", val: null };
		}

		case "KvDeleteRangeRequest": {
			const start = new Uint8Array(data.val.start);
			const end = new Uint8Array(data.val.end);

			if (start.byteLength + 2 > 2048) {
				return {
					tag: "ErrorResponse",
					val: {
						code: "key_too_large",
						message: "start key is too long (max 2048 bytes)",
					},
				};
			}
			if (end.byteLength + 2 > 2048) {
				return {
					tag: "ErrorResponse",
					val: {
						code: "key_too_large",
						message: "end key is too long (max 2048 bytes)",
					},
				};
			}

			await managerDriver.kvDeleteRange(actorId, start, end);

			return { tag: "KvDeleteResponse", val: null };
		}

		default: {
			// Should never happen since processRequestData routes only KV tags here.
			const _exhaustive: never = data;
			throw new Error(`unexpected request tag`);
		}
	}
}

function handleToServerMessage(
	conn: KvChannelConnection,
	ws: WSContext,
	managerDriver: ManagerDriver,
	msg: ToServer,
): void {
	switch (msg.tag) {
		case "ToServerRequest":
			// Fire-and-forget: the handler sends the response itself.
			handleRequest(conn, ws, managerDriver, msg.val).catch(
				(err) => {
					logger().error({
						msg: "unhandled error in kv channel request handler",
						error:
							err instanceof Error ? err.message : String(err),
					});
				},
			);
			break;

		case "ToServerPong":
			conn.lastPongTs = Date.now();
			break;
	}
}

/** Validate the protocol version query parameter. Returns an error string or null. */
export function validateProtocolVersion(
	protocolVersion: string | undefined,
): string | null {
	if (!protocolVersion) {
		return "missing protocol_version query parameter";
	}
	const version = Number.parseInt(protocolVersion, 10);
	if (Number.isNaN(version) || version !== PROTOCOL_VERSION) {
		return `unsupported protocol_version: ${protocolVersion} (server supports ${PROTOCOL_VERSION})`;
	}
	return null;
}

/** Build UpgradeWebSocketArgs for the KV channel endpoint. */
export function createKvChannelWebSocketHandler(
	managerDriver: ManagerDriver,
): {
	onOpen: (event: any, ws: WSContext) => void;
	onMessage: (event: any, ws: WSContext) => void;
	onClose: (event: any, ws: WSContext) => void;
	onError: (error: any, ws: WSContext) => void;
} {
	const conn: KvChannelConnection = {
		openActors: new Set(),
		pingInterval: null,
		pongTimeout: null,
		lastPongTs: Date.now(),
		closed: false,
		ws: null,
	};

	return {
		onOpen: (_event: any, ws: WSContext) => {
			logger().debug({ msg: "kv channel websocket opened" });
			conn.ws = ws;
			startPingPong(conn);
		},

		onMessage: (event: any, ws: WSContext) => {
			try {
				let bytes: Uint8Array;
				if (event.data instanceof ArrayBuffer) {
					bytes = new Uint8Array(event.data);
				} else if (event.data instanceof Uint8Array) {
					bytes = event.data;
				} else if (Buffer.isBuffer(event.data)) {
					bytes = new Uint8Array(event.data);
				} else {
					logger().warn({
						msg: "kv channel received non-binary message, ignoring",
					});
					return;
				}

				const msg = decodeToServer(bytes);
				handleToServerMessage(conn, ws, managerDriver, msg);
			} catch (err: unknown) {
				logger().error({
					msg: "kv channel failed to decode message",
					error: err instanceof Error ? err.message : String(err),
				});
			}
		},

		onClose: (_event: any, _ws: WSContext) => {
			logger().debug({ msg: "kv channel websocket closed" });
			cleanupConnection(conn);
		},

		onError: (error: any, _ws: WSContext) => {
			logger().error({
				msg: "kv channel websocket error",
				error: error instanceof Error ? error.message : String(error),
			});
			cleanupConnection(conn);
		},
	};
}
