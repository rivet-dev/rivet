import * as cbor from "cbor-x";
import type * as protocol from "@/schemas/client-protocol/mod";
import { TO_CLIENT_VERSIONED } from "@/schemas/client-protocol/versioned";
import {
	type ToClient as ToClientJson,
	ToClientSchema,
} from "@/schemas/client-protocol-zod/mod";
import { arrayBuffersEqual, bufferToArrayBuffer } from "@/utils";
import type { AnyDatabaseProvider } from "../database";
import {
	ACTOR_INSTANCE_PERSIST_SYMBOL,
	type ActorInstance,
} from "../instance/mod";
import type { PersistedConn } from "../instance/persisted";
import { CachedSerializer } from "../protocol/serde";
import type { ConnDriver } from "./driver";
import { StateManager } from "./state-manager";

export function generateConnRequestId(): string {
	return crypto.randomUUID();
}

export type ConnId = string;

export type AnyConn = Conn<any, any, any, any, any, any>;

export const CONN_PERSIST_SYMBOL = Symbol("persist");
export const CONN_DRIVER_SYMBOL = Symbol("driver");
export const CONN_ACTOR_SYMBOL = Symbol("actor");
export const CONN_STATE_ENABLED_SYMBOL = Symbol("stateEnabled");
export const CONN_PERSIST_RAW_SYMBOL = Symbol("persistRaw");
export const CONN_HAS_CHANGES_SYMBOL = Symbol("hasChanges");
export const CONN_MARK_SAVED_SYMBOL = Symbol("markSaved");
export const CONN_SEND_MESSAGE_SYMBOL = Symbol("sendMessage");

/**
 * Represents a client connection to a actor.
 *
 * Manages connection-specific data and controls the connection lifecycle.
 *
 * @see {@link https://rivet.dev/docs/connections|Connection Documentation}
 */
export class Conn<S, CP, CS, V, I, DB extends AnyDatabaseProvider> {
	subscriptions: Set<string> = new Set<string>();

	// TODO: Remove this cyclical reference
	#actor: ActorInstance<S, CP, CS, V, I, DB>;

	// MARK: - Managers
	#stateManager!: StateManager<CP, CS>;

	/**
	 * If undefined, then nothing is connected to this.
	 */
	[CONN_DRIVER_SYMBOL]?: ConnDriver;

	// MARK: - Public Getters

	get [CONN_ACTOR_SYMBOL](): ActorInstance<S, CP, CS, V, I, DB> {
		return this.#actor;
	}

	get [CONN_PERSIST_SYMBOL](): PersistedConn<CP, CS> {
		return this.#stateManager.persist;
	}

	get params(): CP {
		return this.#stateManager.params;
	}

	get [CONN_STATE_ENABLED_SYMBOL](): boolean {
		return this.#stateManager.stateEnabled;
	}

	/**
	 * Gets the current state of the connection.
	 *
	 * Throws an error if the state is not enabled.
	 */
	get state(): CS {
		return this.#stateManager.state;
	}

	/**
	 * Sets the state of the connection.
	 *
	 * Throws an error if the state is not enabled.
	 */
	set state(value: CS) {
		this.#stateManager.state = value;
	}

	/**
	 * Unique identifier for the connection.
	 */
	get id(): ConnId {
		return this.#stateManager.persist.connId;
	}

	/**
	 * @experimental
	 *
	 * If the underlying connection can hibernate.
	 */
	get isHibernatable(): boolean {
		const hibernatableRequestId =
			this.#stateManager.persist.hibernatableRequestId;
		if (!hibernatableRequestId) {
			return false;
		}
		return (
			(this.#actor as any)[
				ACTOR_INSTANCE_PERSIST_SYMBOL
			].hibernatableConns.findIndex((conn: any) =>
				arrayBuffersEqual(
					conn.hibernatableRequestId,
					hibernatableRequestId,
				),
			) > -1
		);
	}

	/**
	 * Timestamp of the last time the connection was seen, i.e. the last time the connection was active and checked for liveness.
	 */
	get lastSeen(): number {
		return this.#stateManager.persist.lastSeen;
	}

	/**
	 * Initializes a new instance of the Connection class.
	 *
	 * This should only be constructed by {@link Actor}.
	 *
	 * @protected
	 */
	constructor(
		actor: ActorInstance<S, CP, CS, V, I, DB>,
		persist: PersistedConn<CP, CS>,
	) {
		this.#actor = actor;
		this.#stateManager = new StateManager(this);
		this.#stateManager.initPersistProxy(persist);
	}

	/**
	 * Returns whether this connection has unsaved changes
	 */
	[CONN_HAS_CHANGES_SYMBOL](): boolean {
		return this.#stateManager.hasChanges();
	}

	/**
	 * Marks changes as saved
	 */
	[CONN_MARK_SAVED_SYMBOL]() {
		this.#stateManager.markSaved();
	}

	/**
	 * Gets the raw persist data for serialization
	 */
	get [CONN_PERSIST_RAW_SYMBOL](): PersistedConn<CP, CS> {
		return this.#stateManager.persistRaw;
	}

	[CONN_SEND_MESSAGE_SYMBOL](message: CachedSerializer<any, any, any>) {
		if (this[CONN_DRIVER_SYMBOL]) {
			const driver = this[CONN_DRIVER_SYMBOL];
			if (driver.sendMessage) {
				driver.sendMessage(this.#actor, this, message);
			} else {
				this.#actor.rLog.debug({
					msg: "conn driver does not support sending messages",
					conn: this.id,
				});
			}
		} else {
			this.#actor.rLog.warn({
				msg: "missing connection driver state for send message",
				conn: this.id,
			});
		}
	}

	/**
	 * Sends an event with arguments to the client.
	 *
	 * @param eventName - The name of the event.
	 * @param args - The arguments for the event.
	 * @see {@link https://rivet.dev/docs/events|Events Documentation}
	 */
	send(eventName: string, ...args: unknown[]) {
		this.#actor.inspector.emitter.emit("eventFired", {
			type: "event",
			eventName,
			args,
			connId: this.id,
		});
		const eventData = { name: eventName, args };
		this[CONN_SEND_MESSAGE_SYMBOL](
			new CachedSerializer(
				eventData,
				TO_CLIENT_VERSIONED,
				ToClientSchema,
				// JSON: args is the raw value (array of arguments)
				(value): ToClientJson => ({
					body: {
						tag: "Event" as const,
						val: {
							name: value.name,
							args: value.args,
						},
					},
				}),
				// BARE/CBOR: args needs to be CBOR-encoded to ArrayBuffer
				(value): protocol.ToClient => ({
					body: {
						tag: "Event" as const,
						val: {
							name: value.name,
							args: bufferToArrayBuffer(cbor.encode(value.args)),
						},
					},
				}),
			),
		);
	}

	/**
	 * Disconnects the client with an optional reason.
	 *
	 * @param reason - The reason for disconnection.
	 */
	async disconnect(reason?: string) {
		if (this[CONN_DRIVER_SYMBOL]) {
			const driver = this[CONN_DRIVER_SYMBOL];
			if (driver.disconnect) {
				driver.disconnect(this.#actor, this, reason);
			} else {
				this.#actor.rLog.debug({
					msg: "no disconnect handler for conn driver",
					conn: this.id,
				});
			}

			this.#actor.connDisconnected(this, true);
		} else {
			this.#actor.rLog.warn({
				msg: "missing connection driver state for disconnect",
				conn: this.id,
			});
		}

		this[CONN_DRIVER_SYMBOL] = undefined;
	}
}
