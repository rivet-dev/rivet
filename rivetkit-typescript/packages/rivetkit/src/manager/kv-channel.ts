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
import { KvStorageQuotaExceededError } from "@/drivers/file-system/kv-limits";
import type { ManagerDriver } from "./driver";
import { logger } from "./log";

// Ping every 3 seconds, close if no pong within 15 seconds.
// Matches runner protocol defaults (runner_update_ping_interval_ms=3000,
// runner_ping_timeout_ms=15000 in engine/packages/config/src/config/pegboard.rs).
const PING_INTERVAL_MS = 3_000;
const PONG_TIMEOUT_MS = 15_000;

// Maximum actors a single connection can open. Prevents unbounded memory growth.
const MAX_ACTORS_PER_CONNECTION = 1_000;

// Sweep interval for removing stale lock entries from dead connections.
const STALE_LOCK_SWEEP_INTERVAL_MS = 60_000;

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

	/** Per-actor request queues for sequential execution. */
	actorQueues: Map<string, Promise<void>>;
}

/** Instance-scoped state for a KV channel manager. */
interface KvChannelManagerState {
	actorLocks: Map<string, KvChannelConnection>;
	activeConnections: Set<KvChannelConnection>;
	staleLockSweepTimer: ReturnType<typeof setInterval> | null;
}

/** Return type of createKvChannelManager. */
export interface KvChannelManager {
	createHandler: (managerDriver: ManagerDriver) => {
		onOpen: (event: any, ws: WSContext) => void;
		onMessage: (event: any, ws: WSContext) => void;
		onClose: (event: any, ws: WSContext) => void;
		onError: (error: any, ws: WSContext) => void;
	};
	shutdown: () => void;
	_testForceCloseAllKvChannels: () => number;
}

/**
 * Create an instance-scoped KV channel manager.
 *
 * All lock state and timers are scoped to the returned object, so multiple
 * manager instances in the same process (e.g., tests) do not share state.
 */
export function createKvChannelManager(): KvChannelManager {
	const state: KvChannelManagerState = {
		actorLocks: new Map(),
		activeConnections: new Set(),
		staleLockSweepTimer: null,
	};

	return {
		createHandler(managerDriver: ManagerDriver) {
			const conn: KvChannelConnection = {
				openActors: new Set(),
				pingInterval: null,
				pongTimeout: null,
				lastPongTs: Date.now(),
				closed: false,
				ws: null,
				actorQueues: new Map(),
			};

			state.activeConnections.add(conn);

			return {
				onOpen: (_event: any, ws: WSContext) => {
					logger().debug({ msg: "kv channel websocket opened" });
					conn.ws = ws;
					startPingPong(state, conn);
				},

				onMessage: (event: any, _ws: WSContext) => {
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
						handleToServerMessage(state, conn, managerDriver, msg);
					} catch (err: unknown) {
						logger().error({
							msg: "kv channel failed to decode message",
							error:
								err instanceof Error
									? err.message
									: String(err),
						});
					}
				},

				onClose: (_event: any, _ws: WSContext) => {
					logger().debug({ msg: "kv channel websocket closed" });
					cleanupConnection(state, conn);
				},

				onError: (error: any, _ws: WSContext) => {
					logger().error({
						msg: "kv channel websocket error",
						error:
							error instanceof Error
								? error.message
								: String(error),
					});
					cleanupConnection(state, conn);
				},
			};
		},

		shutdown() {
			if (state.staleLockSweepTimer) {
				clearInterval(state.staleLockSweepTimer);
				state.staleLockSweepTimer = null;
			}
			state.actorLocks.clear();
			state.activeConnections.clear();
		},

		_testForceCloseAllKvChannels() {
			let closed = 0;
			for (const conn of state.activeConnections) {
				if (!conn.closed && conn.ws) {
					const ws = conn.ws;
					cleanupConnection(state, conn);
					ws.close(1001, "test force disconnect");
					closed++;
				}
			}
			return closed;
		},
	};
}

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

function sendMessage(conn: KvChannelConnection, msg: ToClient): void {
	if (conn.closed || !conn.ws) return;
	const bytes = encodeToClient(msg);
	// Copy to a fresh ArrayBuffer to satisfy WSContext.send() parameter type.
	const copy = new ArrayBuffer(bytes.byteLength);
	new Uint8Array(copy).set(bytes);
	conn.ws.send(copy);
}

function startPingPong(
	state: KvChannelManagerState,
	conn: KvChannelConnection,
): void {
	conn.lastPongTs = Date.now();

	conn.pingInterval = setInterval(() => {
		if (conn.closed || !conn.ws) return;

		const ts = BigInt(Date.now());
		sendMessage(conn, {
			tag: "ToClientPing",
			val: { ts },
		});

		// Check if the last pong was too long ago.
		if (Date.now() - conn.lastPongTs > PONG_TIMEOUT_MS) {
			logger().warn({
				msg: "kv channel pong timeout, closing connection",
			});
			// Capture ws before cleanup nulls it.
			const ws = conn.ws;
			cleanupConnection(state, conn);
			if (ws) {
				ws.close(1000, "pong timeout");
			}
		}
	}, PING_INTERVAL_MS);
}

function cleanupConnection(
	state: KvChannelManagerState,
	conn: KvChannelConnection,
): void {
	conn.closed = true;
	conn.ws = null;
	state.activeConnections.delete(conn);

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
		if (state.actorLocks.get(actorId) === conn) {
			state.actorLocks.delete(actorId);
		}
	}
	conn.openActors.clear();
}

async function handleRequest(
	state: KvChannelManagerState,
	conn: KvChannelConnection,
	managerDriver: ManagerDriver,
	request: ToServerRequest,
): Promise<void> {
	const { requestId, actorId, data } = request;

	try {
		const responseData = await processRequestData(
			state,
			conn,
			managerDriver,
			actorId,
			data,
		);
		sendMessage(conn, makeResponse(requestId, responseData));
	} catch (err: unknown) {
		// Log the full error server-side but return a generic message to the
		// client to avoid leaking internal details. Specific known error codes
		// (actor_not_open, actor_locked, storage_quota_exceeded, etc.) are
		// returned as structured responses before reaching this catch block.
		logger().error({
			msg: "kv channel request error",
			requestId,
			actorId,
			error: err instanceof Error ? err.message : String(err),
		});
		sendMessage(
			conn,
			makeErrorResponse(requestId, "internal_error", "internal error"),
		);
	}
}

// Defense-in-depth: in the engine KV channel, resolve_actor verifies the actor
// belongs to the authenticated namespace. The local dev manager is
// single-namespace, so all actors implicitly belong to the same namespace and
// no cross-namespace access is possible. If a less-privileged auth mechanism is
// introduced for the dev manager, namespace verification should be added here.
async function processRequestData(
	state: KvChannelManagerState,
	conn: KvChannelConnection,
	managerDriver: ManagerDriver,
	actorId: string,
	data: RequestData,
): Promise<ResponseData> {
	switch (data.tag) {
		case "ActorOpenRequest":
			return handleActorOpen(state, conn, actorId);

		case "ActorCloseRequest":
			return handleActorClose(state, conn, actorId);

		case "KvGetRequest":
		case "KvPutRequest":
		case "KvDeleteRequest":
		case "KvDeleteRangeRequest": {
			// All KV operations require the actor to be open on this connection.
			const lockHolder = state.actorLocks.get(actorId);
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
	state: KvChannelManagerState,
	conn: KvChannelConnection,
	actorId: string,
): ResponseData {
	// Reject if this connection already has too many actors open.
	if (conn.openActors.size >= MAX_ACTORS_PER_CONNECTION) {
		return {
			tag: "ErrorResponse",
			val: {
				code: "too_many_actors",
				message: `connection has too many open actors (max ${MAX_ACTORS_PER_CONNECTION})`,
			},
		};
	}

	const existingLock = state.actorLocks.get(actorId);
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

	state.actorLocks.set(actorId, conn);
	conn.openActors.add(actorId);

	// Start the stale lock sweep if not already running.
	ensureStaleLockSweep(state);

	return { tag: "ActorOpenResponse", val: null };
}

function handleActorClose(
	state: KvChannelManagerState,
	conn: KvChannelConnection,
	actorId: string,
): ResponseData {
	if (state.actorLocks.get(actorId) === conn) {
		state.actorLocks.delete(actorId);
	}
	conn.openActors.delete(actorId);

	return { tag: "ActorCloseResponse", val: null };
}

/** Start the stale lock sweep if not already running. */
function ensureStaleLockSweep(state: KvChannelManagerState): void {
	if (state.staleLockSweepTimer) return;
	state.staleLockSweepTimer = setInterval(() => {
		let removed = 0;
		for (const [actorId, conn] of state.actorLocks) {
			if (conn.closed) {
				state.actorLocks.delete(actorId);
				removed++;
			}
		}
		if (removed > 0) {
			logger().debug({
				msg: "kv channel stale lock sweep completed",
				removedCount: removed,
				remainingCount: state.actorLocks.size,
			});
		}
		// Stop the sweep if there are no more lock entries.
		if (state.actorLocks.size === 0 && state.staleLockSweepTimer) {
			clearInterval(state.staleLockSweepTimer);
			state.staleLockSweepTimer = null;
		}
	}, STALE_LOCK_SWEEP_INTERVAL_MS);
	// Allow the process to exit even if the sweep timer is still running.
	state.staleLockSweepTimer.unref?.();
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
				if (err instanceof KvStorageQuotaExceededError) {
					return {
						tag: "ErrorResponse",
						val: {
							code: "storage_quota_exceeded",
							message: err.message,
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
	state: KvChannelManagerState,
	conn: KvChannelConnection,
	managerDriver: ManagerDriver,
	msg: ToServer,
): void {
	switch (msg.tag) {
		case "ToServerRequest": {
			const { actorId } = msg.val;

			// Chain requests per actor so they execute sequentially,
			// preventing journal write ordering violations. Cross-actor
			// requests still execute concurrently since each actor has its
			// own queue. See docs-internal/engine/NATIVE_SQLITE_REVIEW_FIXES.md H2.
			const prev = conn.actorQueues.get(actorId) ?? Promise.resolve();
			const next = prev.then(() =>
				handleRequest(state, conn, managerDriver, msg.val).catch(
					(err) => {
						logger().error({
							msg: "unhandled error in kv channel request handler",
							error:
								err instanceof Error
									? err.message
									: String(err),
						});
					},
				),
			);
			conn.actorQueues.set(actorId, next);

			// Clean up the queue entry once it settles to avoid unbounded map growth.
			next.then(() => {
				if (conn.actorQueues.get(actorId) === next) {
					conn.actorQueues.delete(actorId);
				}
			});
			break;
		}

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
