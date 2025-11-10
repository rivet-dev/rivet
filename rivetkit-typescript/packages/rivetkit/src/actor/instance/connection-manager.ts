import * as cbor from "cbor-x";
import invariant from "invariant";
import { TO_CLIENT_VERSIONED } from "@/schemas/client-protocol/versioned";
import { ToClientSchema } from "@/schemas/client-protocol-zod/mod";
import { arrayBuffersEqual, stringifyError } from "@/utils";
import type { ConnDriver } from "../conn/driver";
import {
	CONN_CONNECTED_SYMBOL,
	CONN_DRIVER_SYMBOL,
	CONN_MARK_SAVED_SYMBOL,
	CONN_PERSIST_RAW_SYMBOL,
	CONN_PERSIST_SYMBOL,
	CONN_SEND_MESSAGE_SYMBOL,
	CONN_SPEAKS_RIVETKIT_SYMBOL,
	Conn,
	type ConnId,
} from "../conn/mod";
import { CreateConnStateContext } from "../contexts/create-conn-state";
import { OnBeforeConnectContext } from "../contexts/on-before-connect";
import { OnConnectContext } from "../contexts/on-connect";
import type { AnyDatabaseProvider } from "../database";
import { CachedSerializer } from "../protocol/serde";
import { deadline } from "../utils";
import { makeConnKey } from "./kv";
import type { ActorInstance } from "./mod";
import type { PersistedConn } from "./persisted";

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
> {
	#actor: ActorInstance<S, CP, CS, V, I, DB>;
	#connections = new Map<ConnId, Conn<S, CP, CS, V, I, DB>>();
	#changedConnections = new Set<ConnId>();

	constructor(actor: ActorInstance<S, CP, CS, V, I, DB>) {
		this.#actor = actor;
	}

	// MARK: - Public API

	get connections(): Map<ConnId, Conn<S, CP, CS, V, I, DB>> {
		return this.#connections;
	}

	get changedConnections(): Set<ConnId> {
		return this.#changedConnections;
	}

	clearChangedConnections() {
		this.#changedConnections.clear();
	}

	getConnForId(id: string): Conn<S, CP, CS, V, I, DB> | undefined {
		return this.#connections.get(id);
	}

	markConnChanged(conn: Conn<S, CP, CS, V, I, DB>) {
		this.#changedConnections.add(conn.id);
		this.#actor.rLog.debug({
			msg: "marked connection as changed",
			connId: conn.id,
			totalChanged: this.#changedConnections.size,
		});
	}

	// MARK: - Connection Lifecycle
	/**
	 * Handles pre-connection logic (i.e. auth & create state) before actually connecting the connection.
	 */
	async prepareConn(
		driver: ConnDriver,
		params: CP,
		request: Request | undefined,
	): Promise<Conn<S, CP, CS, V, I, DB>> {
		this.#actor.assertReady();

		// Check for hibernatable websocket reconnection
		if (driver.requestIdBuf && driver.hibernatable) {
			const existingConn = this.#findHibernatableConn(
				driver.requestIdBuf,
			);

			if (existingConn) {
				return this.#reconnectHibernatableConn(existingConn, driver);
			}
		}

		// Create new connection
		const persist = this.#actor.persist;
		if (this.#actor.config.onBeforeConnect) {
			const ctx = new OnBeforeConnectContext(this.#actor, request);
			await this.#actor.config.onBeforeConnect(ctx, params);
		}

		// Create connection state if enabled
		let connState: CS | undefined;
		if (this.#actor.connStateEnabled) {
			connState = await this.#createConnState(params, request);
		}

		// Create connection persist data
		const connPersist: PersistedConn<CP, CS> = {
			connId: crypto.randomUUID(),
			params: params,
			state: connState as CS,
			lastSeen: Date.now(),
			subscriptions: [],
		};

		// Check if hibernatable
		if (driver.requestIdBuf) {
			const isHibernatable = this.#isHibernatableRequest(
				driver.requestIdBuf,
			);
			if (isHibernatable) {
				connPersist.hibernatableRequestId = driver.requestIdBuf;
			}
		}

		// Create connection instance
		const conn = new Conn<S, CP, CS, V, I, DB>(this.#actor, connPersist);
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
	connectConn(conn: Conn<S, CP, CS, V, I, DB>) {
		invariant(!this.#connections.has(conn.id), "conn already connected");

		this.#connections.set(conn.id, conn);

		this.#changedConnections.add(conn.id);

		this.#callOnConnect(conn);

		this.#actor.inspector.emitter.emit("connectionUpdated");

		this.#actor.resetSleepTimer();

		conn[CONN_CONNECTED_SYMBOL] = true;

		// Send init message
		if (conn[CONN_SPEAKS_RIVETKIT_SYMBOL]) {
			const initData = { actorId: this.#actor.id, connectionId: conn.id };
			conn[CONN_SEND_MESSAGE_SYMBOL](
				new CachedSerializer(
					initData,
					TO_CLIENT_VERSIONED,
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

	/**
	 * Handle connection disconnection.
	 *
	 * This is called by `Conn.disconnect`. This should not call `Conn.disconnect.`
	 */
	async connDisconnected(conn: Conn<S, CP, CS, V, I, DB>) {
		// Remove from tracking
		this.#connections.delete(conn.id);
		this.#changedConnections.delete(conn.id);
		this.#actor.rLog.debug({ msg: "removed conn", connId: conn.id });

		for (const eventName of [...conn.subscriptions.values()]) {
			this.#actor.eventManager.removeSubscription(eventName, conn, true);
		}

		this.#actor.resetSleepTimer();

		this.#actor.inspector.emitter.emit("connectionUpdated");

		// Trigger disconnect
		if (this.#actor.config.onDisconnect) {
			try {
				const result = this.#actor.config.onDisconnect(
					this.#actor.actorContext,
					conn,
				);
				if (result instanceof Promise) {
					result.catch((error) => {
						this.#actor.rLog.error({
							msg: "error in `onDisconnect`",
							error: stringifyError(error),
						});
					});
				}
			} catch (error) {
				this.#actor.rLog.error({
					msg: "error in `onDisconnect`",
					error: stringifyError(error),
				});
			}
		}

		// Remove from KV storage
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

	/**
	 * Utilify funtion for call sites that don't need a separate prepare and connect phase.
	 */
	async prepareAndConnectConn(
		driver: ConnDriver,
		params: CP,
		request: Request | undefined,
	): Promise<Conn<S, CP, CS, V, I, DB>> {
		const conn = await this.prepareConn(driver, params, request);
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
			const conn = new Conn<S, CP, CS, V, I, DB>(
				this.#actor,
				connPersist,
			);
			this.#connections.set(conn.id, conn);

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

	/**
	 * Gets persistence data for all changed connections.
	 */
	getChangedConnectionsData(): Array<[Uint8Array, Uint8Array]> {
		const entries: Array<[Uint8Array, Uint8Array]> = [];

		for (const connId of this.#changedConnections) {
			const conn = this.#connections.get(connId);
			if (conn) {
				const connData = cbor.encode(conn[CONN_PERSIST_RAW_SYMBOL]);
				entries.push([makeConnKey(connId), connData]);
				conn[CONN_MARK_SAVED_SYMBOL]();
			}
		}

		return entries;
	}

	// MARK: - Private Helpers

	#findHibernatableConn(
		requestIdBuf: ArrayBuffer,
	): Conn<S, CP, CS, V, I, DB> | undefined {
		return Array.from(this.#connections.values()).find(
			(conn) =>
				conn[CONN_PERSIST_SYMBOL].hibernatableRequestId &&
				arrayBuffersEqual(
					conn[CONN_PERSIST_SYMBOL].hibernatableRequestId,
					requestIdBuf,
				),
		);
	}

	#reconnectHibernatableConn(
		existingConn: Conn<S, CP, CS, V, I, DB>,
		driver: ConnDriver,
	): Conn<S, CP, CS, V, I, DB> {
		this.#actor.rLog.debug({
			msg: "reconnecting hibernatable websocket connection",
			connectionId: existingConn.id,
			requestId: driver.requestId,
		});

		// Clean up existing driver state if present
		if (existingConn[CONN_DRIVER_SYMBOL]) {
			this.#cleanupDriverState(existingConn);
		}

		// Update connection with new socket
		existingConn[CONN_DRIVER_SYMBOL] = driver;
		existingConn[CONN_PERSIST_SYMBOL].lastSeen = Date.now();

		this.#actor.inspector.emitter.emit("connectionUpdated");

		return existingConn;
	}

	#cleanupDriverState(conn: Conn<S, CP, CS, V, I, DB>) {
		const driver = conn[CONN_DRIVER_SYMBOL];
		if (driver?.disconnect) {
			driver.disconnect(
				this.#actor,
				conn,
				"Reconnecting hibernatable websocket with new driver state",
			);
		}
	}

	async #createConnState(
		params: CP,
		request: Request | undefined,
	): Promise<CS | undefined> {
		if ("createConnState" in this.#actor.config) {
			const ctx = new CreateConnStateContext(this.#actor, request);
			const dataOrPromise = this.#actor.config.createConnState(
				ctx,
				params,
			);
			if (dataOrPromise instanceof Promise) {
				return await deadline(
					dataOrPromise,
					this.#actor.config.options.createConnStateTimeout,
				);
			}
			return dataOrPromise;
		} else if ("connState" in this.#actor.config) {
			return structuredClone(this.#actor.config.connState);
		}

		throw new Error(
			"Could not create connection state from 'createConnState' or 'connState'",
		);
	}

	#isHibernatableRequest(requestIdBuf: ArrayBuffer): boolean {
		return (
			this.#actor.persist.hibernatableConns.findIndex((conn) =>
				arrayBuffersEqual(conn.hibernatableRequestId, requestIdBuf),
			) !== -1
		);
	}

	#callOnConnect(conn: Conn<S, CP, CS, V, I, DB>) {
		if (this.#actor.config.onConnect) {
			try {
				const ctx = new OnConnectContext(this.#actor, conn);
				const result = this.#actor.config.onConnect(ctx, conn);
				if (result instanceof Promise) {
					deadline(
						result,
						this.#actor.config.options.onConnectTimeout,
					).catch((error) => {
						this.#actor.rLog.error({
							msg: "error in `onConnect`, closing socket",
							error,
						});
						conn?.disconnect("`onConnect` failed");
					});
				}
			} catch (error) {
				this.#actor.rLog.error({
					msg: "error in `onConnect`",
					error: stringifyError(error),
				});
				conn?.disconnect("`onConnect` failed");
			}
		}
	}
}
