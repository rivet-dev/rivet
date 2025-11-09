import * as cbor from "cbor-x";
import { arrayBuffersEqual, idToStr, stringifyError } from "@/utils";
import type { OnConnectOptions } from "../config";
import type { ConnDriver } from "../conn/driver";
import {
	CONN_DRIVER_SYMBOL,
	CONN_PERSIST_SYMBOL,
	Conn,
	type ConnId,
} from "../conn/mod";
import type { AnyDatabaseProvider } from "../database";
import type { ActorDriver } from "../driver";
import { deadline } from "../utils";
import { makeConnKey } from "./kv";
import { ACTOR_INSTANCE_PERSIST_SYMBOL, type ActorInstance } from "./mod";
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
	 * Creates a new connection or reconnects an existing hibernatable connection.
	 */
	async createConn(
		driver: ConnDriver,
		params: CP,
		request?: Request,
	): Promise<Conn<S, CP, CS, V, I, DB>> {
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
		return await this.#createNewConn(driver, params, request);
	}

	/**
	 * Handle connection disconnection.
	 * Clean disconnects remove the connection immediately.
	 * Unclean disconnects keep the connection for potential reconnection.
	 */
	async connDisconnected(
		conn: Conn<S, CP, CS, V, I, DB>,
		wasClean: boolean,
		actorDriver: ActorDriver,
		eventManager: any, // EventManager type
	) {
		if (wasClean) {
			// Clean disconnect - remove immediately
			await this.removeConn(conn, actorDriver, eventManager);
		} else {
			// Unclean disconnect - keep for reconnection
			this.#handleUncleanDisconnect(conn);
		}
	}

	/**
	 * Removes a connection and cleans up its resources.
	 */
	async removeConn(
		conn: Conn<S, CP, CS, V, I, DB>,
		actorDriver: ActorDriver,
		eventManager: any, // EventManager type
	) {
		// Remove from KV storage
		const key = makeConnKey(conn.id);
		try {
			await actorDriver.kvBatchDelete(this.#actor.id, [key]);
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

		// Remove from tracking
		this.#connections.delete(conn.id);
		this.#changedConnections.delete(conn.id);
		this.#actor.rLog.debug({ msg: "removed conn", connId: conn.id });

		// Clean up subscriptions via EventManager
		if (eventManager) {
			for (const eventName of [...conn.subscriptions.values()]) {
				eventManager.removeSubscription(eventName, conn, true);
			}
		}

		// Emit events and call lifecycle hooks
		this.#actor.inspector.emitter.emit("connectionUpdated");

		const config = (this.#actor as any).config;
		if (config?.onDisconnect) {
			try {
				const result = config.onDisconnect(
					this.#actor.actorContext,
					conn,
				);
				if (result instanceof Promise) {
					result.catch((error: any) => {
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
	}

	// MARK: - Persistence

	/**
	 * Restores connections from persisted data during actor initialization.
	 */
	restoreConnections(
		connections: PersistedConn<CP, CS>[],
		eventManager: any, // EventManager type
	) {
		for (const connPersist of connections) {
			// Create connection instance
			const conn = new Conn<S, CP, CS, V, I, DB>(
				this.#actor,
				connPersist,
			);
			this.#connections.set(conn.id, conn);

			// Restore subscriptions
			for (const sub of connPersist.subscriptions) {
				eventManager.addSubscription(sub.eventName, conn, true);
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
				const connData = cbor.encode(conn.persistRaw);
				entries.push([makeConnKey(connId), connData]);
				conn.markSaved();
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

	async #createNewConn(
		driver: ConnDriver,
		params: CP,
		request: Request | undefined,
	): Promise<Conn<S, CP, CS, V, I, DB>> {
		const config = this.#actor.config;
		const persist = (this.#actor as any)[ACTOR_INSTANCE_PERSIST_SYMBOL];
		// Prepare connection state
		let connState: CS | undefined;

		const onBeforeConnectOpts = {
			request,
		} satisfies OnConnectOptions;

		// Call onBeforeConnect hook
		if (config.onBeforeConnect) {
			await config.onBeforeConnect(
				this.#actor.actorContext,
				onBeforeConnectOpts,
				params,
			);
		}

		// Create connection state if enabled
		if ((this.#actor as any).connStateEnabled) {
			connState = await this.#createConnState(
				config,
				onBeforeConnectOpts,
				params,
			);
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
				persist,
			);
			if (isHibernatable) {
				connPersist.hibernatableRequestId = driver.requestIdBuf;
			}
		}

		// Create connection instance
		const conn = new Conn<S, CP, CS, V, I, DB>(this.#actor, connPersist);
		conn[CONN_DRIVER_SYMBOL] = driver;
		this.#connections.set(conn.id, conn);

		// Mark as changed for persistence
		this.#changedConnections.add(conn.id);

		// Call onConnect lifecycle hook
		if (config.onConnect) {
			this.#callOnConnect(config, conn);
		}

		this.#actor.inspector.emitter.emit("connectionUpdated");

		return conn;
	}

	async #createConnState(
		config: any,
		opts: OnConnectOptions,
		params: CP,
	): Promise<CS | undefined> {
		if ("createConnState" in config) {
			const dataOrPromise = config.createConnState(
				this.#actor.actorContext,
				opts,
				params,
			);
			if (dataOrPromise instanceof Promise) {
				return await deadline(
					dataOrPromise,
					config.options.createConnStateTimeout,
				);
			}
			return dataOrPromise;
		} else if ("connState" in config) {
			return structuredClone(config.connState);
		}

		throw new Error(
			"Could not create connection state from 'createConnState' or 'connState'",
		);
	}

	#isHibernatableRequest(requestIdBuf: ArrayBuffer, persist: any): boolean {
		return (
			persist.hibernatableConns.findIndex((conn: any) =>
				arrayBuffersEqual(conn.hibernatableRequestId, requestIdBuf),
			) !== -1
		);
	}

	#callOnConnect(config: any, conn: Conn<S, CP, CS, V, I, DB>) {
		try {
			const result = config.onConnect(this.#actor.actorContext, conn);
			if (result instanceof Promise) {
				deadline(result, config.options.onConnectTimeout).catch(
					(error: any) => {
						this.#actor.rLog.error({
							msg: "error in `onConnect`, closing socket",
							error,
						});
						conn?.disconnect("`onConnect` failed");
					},
				);
			}
		} catch (error) {
			this.#actor.rLog.error({
				msg: "error in `onConnect`",
				error: stringifyError(error),
			});
			conn?.disconnect("`onConnect` failed");
		}
	}

	#handleUncleanDisconnect(conn: Conn<S, CP, CS, V, I, DB>) {
		if (!conn[CONN_DRIVER_SYMBOL]) {
			this.#actor.rLog.warn("called conn disconnected without driver");
		}

		// Update last seen for cleanup tracking
		conn[CONN_PERSIST_SYMBOL].lastSeen = Date.now();

		// Remove socket
		conn[CONN_DRIVER_SYMBOL] = undefined;
	}
}
