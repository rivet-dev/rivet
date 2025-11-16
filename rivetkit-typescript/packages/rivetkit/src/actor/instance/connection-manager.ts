import * as cbor from "cbor-x";
import invariant from "invariant";
import { TO_CLIENT_VERSIONED } from "@/schemas/client-protocol/versioned";
import { CONN_VERSIONED } from "@/schemas/actor-persist/versioned";
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
import { CreateConnStateContext } from "../contexts/create-conn-state";
import { OnBeforeConnectContext } from "../contexts/on-before-connect";
import { OnConnectContext } from "../contexts/on-connect";
import type { AnyDatabaseProvider } from "../database";
import { CachedSerializer } from "../protocol/serde";
import { deadline } from "../utils";
import { makeConnKey } from "./kv";
import type { ActorInstance } from "./mod";
import {
	convertConnToBarePersistedConn,
	PersistedConn,
} from "../conn/persisted";
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
		isRestoringHibernatable: boolean,
	): Promise<Conn<S, CP, CS, V, I, DB>> {
		this.#actor.assertReady();

		invariant(
			request?.url.startsWith("http://actor/") ?? true,
			"request must start with `http://actor/`",
		);

		// Check for hibernatable websocket reconnection
		if (isRestoringHibernatable) {
			const existingConn = this.findHibernatableConn(driver.requestIdBuf);
			invariant(
				existingConn,
				"cannot find connection for restoring connection",
			);
			return this.#reconnectHibernatableConn(existingConn, driver);
		}

		// Create new connection
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
		const hibernatable = driver.hibernatable;
		invariant(
			hibernatable && driver.requestIdBuf,
			"must have requestIdBuf if hibernatable",
		);
		throw "TODO";
		// TODO:
		// const connPersist: PersistedConn<CP, CS> = {
		//           id: crypto.randomUUID(),
		//           parameters: params,
		//           state: connState as CS,
		//           subscriptions: [],
		//           // Fallback to empty buf if not provided since we don't use this value
		//           hibernatableRequestId: driver.hibernatable
		//               ? driver.requestIdBuf
		//               : new ArrayBuffer(),
		//           lastSeenTimestamp: Date.now(),
		//           // First message index will be 1, so we start at 0
		//           msgIndex: 0,
		//           requestPath: "",
		//           requestHeaders: undefined
		//       };

		// // Create connection instance
		// const conn = new Conn<S, CP, CS, V, I, DB>(this.#actor, connPersist);
		// conn[CONN_DRIVER_SYMBOL] = driver;
		//
		// return conn;
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
			this.#disconnectExistingDriver(existingConn);
		}

		// Update connection with new socket
		existingConn[CONN_DRIVER_SYMBOL] = driver;
		existingConn[
			CONN_STATE_MANAGER_SYMBOL
		].hibernatableDataOrError().lastSeenTimestamp = Date.now();

		// Mark as changed for persistence
		this.#changedConnections.add(existingConn.id);

		// Reset sleep timer since we have an active connection
		this.#actor.resetSleepTimer();

		// Mark connection as connected
		existingConn[CONN_CONNECTED_SYMBOL] = true;

		this.#actor.inspector.emitter.emit("connectionUpdated");

		return existingConn;
	}

	#disconnectExistingDriver(conn: Conn<S, CP, CS, V, I, DB>) {
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
	 * Utilify function for call sites that don't need a separate prepare and connect phase.
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
			const conn = new Conn<S, CP, CS, V, I, DB>(this.#actor, {
				hibernatable: connPersist,
			});
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
	getChangedConnectionsKvEntries(): Array<[Uint8Array, Uint8Array]> {
		const entries: Array<[Uint8Array, Uint8Array]> = [];

		for (const connId of this.#changedConnections) {
			const conn = this.#connections.get(connId);
			if (conn) {
				const connStateManager = conn[CONN_STATE_MANAGER_SYMBOL];
				const hibernatableDataRaw =
					connStateManager.hibernatableDataRaw;
				if (hibernatableDataRaw) {
					const bareData = convertConnToBarePersistedConn<CP, CS>(
						hibernatableDataRaw,
					);
					const connData =
						CONN_VERSIONED.serializeWithEmbeddedVersion(bareData);
					entries.push([makeConnKey(connId), connData]);
					connStateManager.markSaved();
				} else {
					this.#actor.log.warn({
						msg: "missing raw hibernatable data for conn in getChangedConnectionsData",
						connId: conn.id,
					});
				}
			}
		}

		return entries;
	}

	// MARK: - Private Helpers

	findHibernatableConn(
		requestIdBuf: ArrayBuffer,
	): Conn<S, CP, CS, V, I, DB> | undefined {
		return Array.from(this.#connections.values()).find((conn) => {
			const connStateManager = conn[CONN_STATE_MANAGER_SYMBOL];
			const connRequestId =
				connStateManager.hibernatableDataRaw?.hibernatableRequestId;
			return (
				connRequestId && arrayBuffersEqual(connRequestId, requestIdBuf)
			);
		});
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
