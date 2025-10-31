import * as cbor from "cbor-x";
import invariant from "invariant";
import { PersistedHibernatableWebSocket } from "@/schemas/actor-persist/mod";
import type * as protocol from "@/schemas/client-protocol/mod";
import { TO_CLIENT_VERSIONED } from "@/schemas/client-protocol/versioned";
import { bufferToArrayBuffer } from "@/utils";
import {
	CONN_DRIVERS,
	ConnDriverKind,
	type ConnDriverState,
	ConnReadyState,
	getConnDriverKindFromState,
} from "./conn-drivers";
import type { ConnSocket } from "./conn-socket";
import type { AnyDatabaseProvider } from "./database";
import * as errors from "./errors";
import type { ActorInstance } from "./instance";
import type { PersistedConn } from "./persisted";
import { CachedSerializer } from "./protocol/serde";
import { generateSecureToken } from "./utils";

export function generateConnId(): string {
	return crypto.randomUUID();
}

export function generateConnToken(): string {
	return generateSecureToken(32);
}

export function generateConnSocketId(): string {
	return crypto.randomUUID();
}

export type ConnId = string;

export type AnyConn = Conn<any, any, any, any, any, any>;

export type ConnectionStatus = "connected" | "reconnecting";

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

	/**
	 * The proxied state that notifies of changes automatically.
	 *
	 * Any data that should be stored indefinitely should be held within this object.
	 */
	__persist: PersistedConn<CP, CS>;

	get __driverState(): ConnDriverState | undefined {
		return this.__socket?.driverState;
	}

	/**
	 * Socket connected to this connection.
	 *
	 * If undefined, then nothing is connected to this.
	 */
	__socket?: ConnSocket;

	get __status(): ConnectionStatus {
		if (this.__socket) {
			return "connected";
		} else {
			return "reconnecting";
		}
	}

	public get params(): CP {
		return this.__persist.params;
	}

	public get __stateEnabled() {
		return this.#actor.connStateEnabled;
	}

	/**
	 * Gets the current state of the connection.
	 *
	 * Throws an error if the state is not enabled.
	 */
	public get state(): CS {
		this.#validateStateEnabled();
		if (!this.__persist.state) throw new Error("state should exists");
		return this.__persist.state;
	}

	/**
	 * Sets the state of the connection.
	 *
	 * Throws an error if the state is not enabled.
	 */
	public set state(value: CS) {
		this.#validateStateEnabled();
		this.__persist.state = value;
	}

	/**
	 * Unique identifier for the connection.
	 */
	public get id(): ConnId {
		return this.__persist.connId;
	}

	/**
	 * Token used to authenticate this request.
	 */
	public get _token(): string {
		return this.__persist.token;
	}

	/**
	 * Status of the connection.
	 */
	public get status(): ConnectionStatus {
		return this.__status;
	}

	/**
	 * @experimental
	 *
	 * If the underlying connection can hibernate.
	 */
	public get isHibernatable(): boolean {
		if (this.__driverState) {
			const driverKind = getConnDriverKindFromState(this.__driverState);
			const driver = CONN_DRIVERS[driverKind];
			return driver.isHibernatable(
				this.#actor,
				this,
				(this.__driverState as any)[driverKind],
			);
		} else {
			return false;
		}
	}

	/**
	 * Timestamp of the last time the connection was seen, i.e. the last time the connection was active and checked for liveness.
	 */
	public get lastSeen(): number {
		return this.__persist.lastSeen;
	}

	/**
	 * Initializes a new instance of the Connection class.
	 *
	 * This should only be constructed by {@link Actor}.
	 *
	 * @protected
	 */
	public constructor(
		actor: ActorInstance<S, CP, CS, V, I, DB>,
		persist: PersistedConn<CP, CS>,
	) {
		this.#actor = actor;
		this.__persist = persist;
	}

	#validateStateEnabled() {
		if (!this.__stateEnabled) {
			throw new errors.ConnStateNotEnabled();
		}
	}

	/**
	 * Sends a WebSocket message to the client.
	 *
	 * @param message - The message to send.
	 *
	 * @protected
	 */
	public _sendMessage(message: CachedSerializer<protocol.ToClient>) {
		if (this.__driverState) {
			const driverKind = getConnDriverKindFromState(this.__driverState);
			const driver = CONN_DRIVERS[driverKind];
			if (driver.sendMessage) {
				driver.sendMessage(
					this.#actor,
					this,
					(this.__driverState as any)[driverKind],
					message,
				);
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
	public send(eventName: string, ...args: unknown[]) {
		this.#actor.inspector.emitter.emit("eventFired", {
			type: "event",
			eventName,
			args,
			connId: this.id,
		});
		this._sendMessage(
			new CachedSerializer<protocol.ToClient>(
				{
					body: {
						tag: "Event",
						val: {
							name: eventName,
							args: bufferToArrayBuffer(cbor.encode(args)),
						},
					},
				},
				TO_CLIENT_VERSIONED,
			),
		);
	}

	/**
	 * Disconnects the client with an optional reason.
	 *
	 * @param reason - The reason for disconnection.
	 */
	public async disconnect(reason?: string) {
		if (this.__socket && this.__driverState) {
			const driverKind = getConnDriverKindFromState(this.__driverState);
			const driver = CONN_DRIVERS[driverKind];
			if (driver.disconnect) {
				driver.disconnect(
					this.#actor,
					this,
					(this.__driverState as any)[driverKind],
					reason,
				);
			} else {
				this.#actor.rLog.debug({
					msg: "no disconnect handler for conn driver",
					conn: this.id,
				});
			}

			this.#actor.__connDisconnected(this, true, this.__socket.socketId);
		} else {
			this.#actor.rLog.warn({
				msg: "missing connection driver state for disconnect",
				conn: this.id,
			});
		}

		this.__socket = undefined;
	}
}
