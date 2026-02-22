import { HibernatingWebSocketMetadata } from "@rivetkit/engine-runner";
import * as cbor from "cbor-x";
import invariant from "invariant";
import { CONN_VERSIONED } from "@/schemas/actor-persist/versioned";
import {
	CURRENT_VERSION as CLIENT_PROTOCOL_CURRENT_VERSION,
	TO_CLIENT_VERSIONED,
} from "@/schemas/client-protocol/versioned";
import { ToClientSchema } from "@/schemas/client-protocol-zod/mod";
import { arrayBuffersEqual, stringifyError } from "@/utils";
import type { ConnDriver } from "../conn/driver";
import {
	CONN_CONNECTED_SYMBOL,
	CONN_DRIVER_SYMBOL,
	CONN_SEND_MESSAGE_SYMBOL,
	CONN_SPEAKS_RIVETKIT_SYMBOL,
	CONN_STATE_MANAGER_SYMBOL,
	Conn,
	type ConnId,
} from "../conn/mod";
import {
	convertConnToBarePersistedConn,
	type PersistedConn,
} from "../conn/persisted";
import type { ConnDataInput } from "../conn/state-manager";
import {
	BeforeConnectContext,
	ConnectContext,
	CreateConnStateContext,
} from "../contexts";
import type { AnyDatabaseProvider } from "../database";
import { CachedSerializer } from "../protocol/serde";
import type { EventSchemaConfig, QueueSchemaConfig } from "../schema";
import { deadline } from "../utils";
import { makeConnKey } from "./keys";
import type { ActorInstance } from "./mod";
/**
 * Manages all connection-related operations for an actor instance.
 * Handles connection creation, tracking, hibernation, and cleanup.
 */
export class ConnectionManager<
	S,
	CP,
	CS,
	V,
	I,
	DB extends AnyDatabaseProvider,
	E extends EventSchemaConfig = Record<never, never>,
	Q extends QueueSchemaConfig = Record<never, never>,
> {
	#actor: ActorInstance<S, CP, CS, V, I, DB, E, Q>;
	#connections = new Map<ConnId, Conn<S, CP, CS, V, I, DB, E, Q>>();
	#pendingDisconnectCount = 0;

	/** Connections that have had their state changed and need to be persisted. */
	#connsWithPersistChanged = new Set<ConnId>();

	constructor(actor: ActorInstance<S, CP, CS, V, I, DB, E, Q>) {
		this.#actor = actor;
	}

	get connections(): Map<ConnId, Conn<S, CP, CS, V, I, DB, E, Q>> {
		return this.#connections;
	}

	getConnForId(id: string): Conn<S, CP, CS, V, I, DB, E, Q> | undefined {
		return this.#connections.get(id);
	}

	get connsWithPersistChanged(): Set<ConnId> {
		return this.#connsWithPersistChanged;
	}

	get pendingDisconnectCount(): number {
		return this.#pendingDisconnectCount;
	}

	clearConnWithPersistChanged() {
		this.#connsWithPersistChanged.clear();
	}

	markConnWithPersistChanged(conn: Conn<S, CP, CS, V, I, DB, E, Q>) {
		invariant(
			conn.isHibernatable,
			"cannot mark non-hibernatable conn for persist",
		);

		this.#actor.rLog.debug({
			msg: "marked connection as changed",
			connId: conn.id,
			totalChanged: this.#connsWithPersistChanged.size,
		});

		this.#connsWithPersistChanged.add(conn.id);

		this.#actor.stateManager.savePersistThrottled();
	}

	// MARK: - Connection Lifecycle
	/**
	 * Handles pre-connection logic (i.e. auth & create state) before actually connecting the connection.
	 */
	async prepareConn(
		driver: ConnDriver,
		params: CP,
		request: Request | undefined,
		requestPath: string | undefined,
		requestHeaders: Record<string, string> | undefined,
		isHibernatable: boolean,
		isRestoringHibernatable: boolean,
	): Promise<Conn<S, CP, CS, V, I, DB, E, Q>> {
		this.#actor.assertReady();

		// TODO: Add back
		// const url = request?.url;
		// invariant(
		// 	url?.startsWith("http://actor/") ?? true,
		// 	`url ${url} must start with 'http://actor/'`,
		// );

		// Check for hibernatable websocket reconnection
		if (isRestoringHibernatable) {
			return this.#reconnectHibernatableConn(driver);
		}

		// Create new connection
		if (this.#actor.config.onBeforeConnect) {
			const ctx = new BeforeConnectContext(this.#actor, request);
			await this.#actor.runInTraceSpan(
				"actor.onBeforeConnect",
				{
					"rivet.conn.type": driver.type,
				},
				() => this.#actor.config.onBeforeConnect!(ctx, params),
			);
		}

		// Create connection state if enabled
		let connState: CS | undefined;
		if (this.#actor.connStateEnabled) {
			connState = await this.#createConnState(params, request);
		}

		// Create connection persist data
		let connData: ConnDataInput<CP, CS>;
		if (isHibernatable) {
			const hibernatable = driver.hibernatable;
			invariant(hibernatable, "must have hibernatable");
			invariant(requestPath, "missing requestPath for hibernatable ws");
			invariant(
				requestHeaders,
				"missing requestHeaders for hibernatable ws",
			);
			connData = {
				hibernatable: {
					id: crypto.randomUUID(),
					parameters: params,
					state: connState as CS,
					subscriptions: [],
					gatewayId: hibernatable.gatewayId,
					requestId: hibernatable.requestId,
					clientMessageIndex: 0,
					// First message index will be 1, so we start at 0
					serverMessageIndex: 0,
					requestPath,
					requestHeaders,
				},
			};
		} else {
			connData = {
				ephemeral: {
					id: crypto.randomUUID(),
					parameters: params,
					state: connState as CS,
				},
			};
		}

		// Create connection instance
		const conn = new Conn<S, CP, CS, V, I, DB, E, Q>(this.#actor, connData);
		conn[CONN_DRIVER_SYMBOL] = driver;

		return conn;
	}

	/**
	 * Adds a connection form prepareConn to the actor and calls onConnect.
	 *
	 * This method is intentionally not async since it needs to be called in
	 * `onOpen` for WebSockets. If this is async, the order of open events will
	 * be messed up and cause race conditions that can drop WebSocket messages.
	 * So all async work in prepareConn.
	 */
	connectConn(conn: Conn<S, CP, CS, V, I, DB, E, Q>) {
		invariant(!this.#connections.has(conn.id), "conn already connected");

		this.#connections.set(conn.id, conn);

		// Notify driver about new connection BEFORE marking as changed
		//
		// This ensures the driver can set up any necessary state (like #hwsMessageIndex)
		// before saveState is triggered by markConnWithPersistChanged
		if (this.#actor.driver.onCreateConn) {
			this.#actor.driver.onCreateConn(conn);
		}

		if (conn.isHibernatable) {
			this.markConnWithPersistChanged(conn);
		}

		this.#callOnConnect(conn);

		this.#actor.inspector.emitter.emit("connectionsUpdated");

		this.#actor.resetSleepTimer();

		conn[CONN_CONNECTED_SYMBOL] = true;

		// Send init message
		if (conn[CONN_SPEAKS_RIVETKIT_SYMBOL]) {
			const initData = { actorId: this.#actor.id, connectionId: conn.id };
			conn[CONN_SEND_MESSAGE_SYMBOL](
				new CachedSerializer(
					initData,
					TO_CLIENT_VERSIONED,
					CLIENT_PROTOCOL_CURRENT_VERSION,
					ToClientSchema,
					// JSON: identity conversion (no nested data to encode)
					(value) => ({
						body: {
							tag: "Init" as const,
							val: value,
						},
					}),
					// BARE/CBOR: identity conversion (no nested data to encode)
					(value) => ({
						body: {
							tag: "Init" as const,
							val: value,
						},
					}),
				),
			);
		}
	}

	#reconnectHibernatableConn(
		driver: ConnDriver,
	): Conn<S, CP, CS, V, I, DB, E, Q> {
		invariant(driver.hibernatable, "missing requestIdBuf");
		const existingConn = this.findHibernatableConn(
			driver.hibernatable.gatewayId,
			driver.hibernatable.requestId,
		);
		invariant(
			existingConn,
			"cannot find connection for restoring connection",
		);

		this.#actor.rLog.debug({
			msg: "reconnecting hibernatable websocket connection",
			connectionId: existingConn.id,
		});

		// Clean up existing driver state if present
		if (existingConn[CONN_DRIVER_SYMBOL]) {
			this.#disconnectExistingDriver(existingConn);
		}

		// Update connection with new socket
		existingConn[CONN_DRIVER_SYMBOL] = driver;

		// Reset sleep timer since we have an active connection
		this.#actor.resetSleepTimer();

		// Mark connection as connected
		existingConn[CONN_CONNECTED_SYMBOL] = true;

		this.#actor.inspector.emitter.emit("connectionsUpdated");

		return existingConn;
	}

	#disconnectExistingDriver(conn: Conn<S, CP, CS, V, I, DB, E, Q>) {
		const driver = conn[CONN_DRIVER_SYMBOL];
		if (driver?.disconnect) {
			driver.disconnect(
				this.#actor,
				conn,
				"Reconnecting hibernatable websocket with new driver state",
			);
		}
	}

	/**
	 * Handle connection disconnection.
	 *
	 * This is called by `Conn.disconnect`. This should not call `Conn.disconnect.`
	 */
	async connDisconnected(conn: Conn<S, CP, CS, V, I, DB, E, Q>) {
		// Remove from tracking
		this.#connections.delete(conn.id);

		this.#actor.rLog.debug({ msg: "removed conn", connId: conn.id });

		// Notify driver about connection removal
		if (this.#actor.driver.onDestroyConn) {
			this.#actor.driver.onDestroyConn(conn);
		}

		for (const eventName of [...conn.subscriptions.values()]) {
			this.#actor.eventManager.removeSubscription(eventName, conn, true);
		}

		this.#actor.inspector.emitter.emit("connectionsUpdated");
		this.#pendingDisconnectCount += 1;

		const attributes = {
			"rivet.conn.id": conn.id,
			"rivet.conn.type": conn[CONN_DRIVER_SYMBOL]?.type,
			"rivet.conn.hibernatable": conn.isHibernatable,
		};
		const span = this.#actor.startTraceSpan(
			"actor.onDisconnect",
			attributes,
		);

		try {
			if (this.#actor.config.onDisconnect) {
				const result = this.#actor.traces.withSpan(span, () =>
					this.#actor.config.onDisconnect!(
						this.#actor.actorContext,
						conn,
					),
				);
				this.#actor.emitTraceEvent(
					"connection.disconnect",
					attributes,
					span,
				);
				if (result instanceof Promise) {
					await result;
				}
				this.#actor.endTraceSpan(span, { code: "OK" });
			} else {
				this.#actor.emitTraceEvent(
					"connection.disconnect",
					attributes,
					span,
				);
				this.#actor.endTraceSpan(span, { code: "OK" });
			}
		} catch (error) {
			this.#actor.endTraceSpan(span, {
				code: "ERROR",
				message: stringifyError(error),
			});
			this.#actor.rLog.error({
				msg: "error in `onDisconnect`",
				error: stringifyError(error),
			});
		} finally {
			// Remove from connsWithPersistChanged after onDisconnect to handle any
			// state changes made during the disconnect callback. Disconnected connections
			// are removed from KV storage via kvBatchDelete below, not through the
			// normal persist save flow, so they should not trigger persist saves.
			this.#connsWithPersistChanged.delete(conn.id);

			// Remove from KV storage.
			if (conn.isHibernatable) {
				const key = makeConnKey(conn.id);
				try {
					await this.#actor.driver.kvBatchDelete(this.#actor.id, [key]);
					this.#actor.rLog.debug({
						msg: "removed connection from KV",
						connId: conn.id,
					});
				} catch (err) {
					this.#actor.rLog.error({
						msg: "kvBatchDelete failed for conn",
						err: stringifyError(err),
					});
				}
			}

			this.#pendingDisconnectCount = Math.max(
				0,
				this.#pendingDisconnectCount - 1,
			);
			this.#actor.resetSleepTimer();
		}
	}

	/**
	 * Utilify function for call sites that don't need a separate prepare and connect phase.
	 */
	async prepareAndConnectConn(
		driver: ConnDriver,
		params: CP,
		request: Request | undefined,
		requestPath: string | undefined,
		requestHeaders: Record<string, string> | undefined,
	): Promise<Conn<S, CP, CS, V, I, DB, E, Q>> {
		const conn = await this.prepareConn(
			driver,
			params,
			request,
			requestPath,
			requestHeaders,
			false,
			false,
		);
		this.connectConn(conn);
		return conn;
	}

	// MARK: - Persistence

	/**
	 * Restores connections from persisted data during actor initialization.
	 */
	restoreConnections(connections: PersistedConn<CP, CS>[]) {
		for (const connPersist of connections) {
			// Create connection instance
			const conn = new Conn<S, CP, CS, V, I, DB, E, Q>(this.#actor, {
				hibernatable: connPersist,
			});
			this.#connections.set(conn.id, conn);

			// Notify driver about restored connection
			if (this.#actor.driver.onCreateConn) {
				this.#actor.driver.onCreateConn(conn);
			}

			// Restore subscriptions
			for (const sub of connPersist.subscriptions) {
				this.#actor.eventManager.addSubscription(
					sub.eventName,
					conn,
					true,
				);
			}
		}
	}

	// MARK: - Private Helpers

	findHibernatableConn(
		gatewayIdBuf: ArrayBuffer,
		requestIdBuf: ArrayBuffer,
	): Conn<S, CP, CS, V, I, DB, E, Q> | undefined {
		return Array.from(this.#connections.values()).find((conn) => {
			const connStateManager = conn[CONN_STATE_MANAGER_SYMBOL];
			const h = connStateManager.hibernatableDataRaw;
			return (
				h &&
				arrayBuffersEqual(h.gatewayId, gatewayIdBuf) &&
				arrayBuffersEqual(h.requestId, requestIdBuf)
			);
		});
	}

	async #createConnState(
		params: CP,
		request: Request | undefined,
	): Promise<CS | undefined> {
		if ("createConnState" in this.#actor.config) {
			const createConnState = this.#actor.config.createConnState;
			const ctx = new CreateConnStateContext(this.#actor, request);
			return await this.#actor.runInTraceSpan(
				"actor.createConnState",
				undefined,
				() => {
					const dataOrPromise = createConnState!(ctx, params);
					if (dataOrPromise instanceof Promise) {
						return deadline(
							dataOrPromise,
							this.#actor.config.options.createConnStateTimeout,
						);
					}
					return dataOrPromise;
				},
			);
		} else if ("connState" in this.#actor.config) {
			return structuredClone(this.#actor.config.connState);
		}

		throw new Error(
			"Could not create connection state from 'createConnState' or 'connState'",
		);
	}

	#callOnConnect(conn: Conn<S, CP, CS, V, I, DB, E, Q>) {
		const attributes = {
			"rivet.conn.id": conn.id,
			"rivet.conn.type": conn[CONN_DRIVER_SYMBOL]?.type,
			"rivet.conn.hibernatable": conn.isHibernatable,
		};
		const span = this.#actor.startTraceSpan("actor.onConnect", attributes);

		try {
			if (this.#actor.config.onConnect) {
				const ctx = new ConnectContext(this.#actor, conn);
				const result = this.#actor.traces.withSpan(span, () =>
					this.#actor.config.onConnect!(ctx, conn),
				);
				this.#actor.emitTraceEvent(
					"connection.connect",
					attributes,
					span,
				);
				if (result instanceof Promise) {
					deadline(
						result,
						this.#actor.config.options.onConnectTimeout,
					)
						.then(() => {
							this.#actor.endTraceSpan(span, { code: "OK" });
						})
						.catch((error) => {
							this.#actor.endTraceSpan(span, {
								code: "ERROR",
								message: stringifyError(error),
							});
							this.#actor.rLog.error({
								msg: "error in `onConnect`, closing socket",
								error,
							});
							conn?.disconnect("`onConnect` failed");
						});
					return;
				}
			}

			this.#actor.emitTraceEvent("connection.connect", attributes, span);
			this.#actor.endTraceSpan(span, { code: "OK" });
		} catch (error) {
			this.#actor.endTraceSpan(span, {
				code: "ERROR",
				message: stringifyError(error),
			});
			this.#actor.rLog.error({
				msg: "error in `onConnect`",
				error: stringifyError(error),
			});
			conn?.disconnect("`onConnect` failed");
		}
	}
}
