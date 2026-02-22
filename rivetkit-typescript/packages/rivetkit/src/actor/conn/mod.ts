import * as cbor from "cbor-x";
import type * as protocol from "@/schemas/client-protocol/mod";
import {
	CURRENT_VERSION as CLIENT_PROTOCOL_CURRENT_VERSION,
	TO_CLIENT_VERSIONED,
} from "@/schemas/client-protocol/versioned";
import {
	type ToClient as ToClientJson,
	ToClientSchema,
} from "@/schemas/client-protocol-zod/mod";
import { bufferToArrayBuffer } from "@/utils";
import type { AnyDatabaseProvider } from "../database";
import { EventPayloadInvalid, InternalError } from "../errors";
import type { ActorInstance } from "../instance/mod";
import { CachedSerializer } from "../protocol/serde";
import {
	type EventSchemaConfig,
	hasSchemaConfigKey,
	type InferEventArgs,
	type InferSchemaMap,
	type QueueSchemaConfig,
	validateSchemaSync,
} from "../schema";
import type { ConnDriver } from "./driver";
import { type ConnDataInput, StateManager } from "./state-manager";

export type ConnId = string;

export type AnyConn = Conn<any, any, any, any, any, any, any, any>;

export const CONN_CONNECTED_SYMBOL = Symbol("connected");
export const CONN_SPEAKS_RIVETKIT_SYMBOL = Symbol("speaksRivetKit");
export const CONN_DRIVER_SYMBOL = Symbol("driver");
export const CONN_ACTOR_SYMBOL = Symbol("actor");
export const CONN_STATE_MANAGER_SYMBOL = Symbol("stateManager");
export const CONN_SEND_MESSAGE_SYMBOL = Symbol("sendMessage");

/**
 * Represents a client connection to a actor.
 *
 * Manages connection-specific data and controls the connection lifecycle.
 *
 * @see {@link https://rivet.dev/docs/connections|Connection Documentation}
 */
export class Conn<
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

	get [CONN_ACTOR_SYMBOL](): ActorInstance<S, CP, CS, V, I, DB, E, Q> {
		return this.#actor;
	}

	#stateManager!: StateManager<CP, CS>;

	get [CONN_STATE_MANAGER_SYMBOL]() {
		return this.#stateManager;
	}

	/**
	 * Connections exist before being connected to an actor. If true, this
	 * connection has been connected.
	 **/
	[CONN_CONNECTED_SYMBOL] = false;

	/**
	 * If undefined, then no socket is connected to this conn
	 */
	[CONN_DRIVER_SYMBOL]?: ConnDriver;

	/**
	 * If this connection is speaking the RivetKit protocol. If false, this is
	 * a raw connection for WebSocket or fetch or inspector.
	 **/
	get [CONN_SPEAKS_RIVETKIT_SYMBOL](): boolean {
		return this[CONN_DRIVER_SYMBOL]?.rivetKitProtocol !== undefined;
	}

	subscriptions: Set<string> = new Set<string>();

	#assertConnected() {
		if (!this[CONN_CONNECTED_SYMBOL])
			throw new InternalError(
				"Connection not connected yet. This happens when trying to use the connection in onBeforeConnect or createConnState.",
			);
	}

	// MARK: - Public Getters
	get params(): CP {
		return this.#stateManager.ephemeralData.parameters;
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
		return this.#stateManager.ephemeralData.id;
	}

	/**
	 * @experimental
	 *
	 * If the underlying connection can hibernate.
	 */
	get isHibernatable(): boolean {
		return this.#stateManager.hibernatableDataRaw !== undefined;
	}

	/**
	 * Initializes a new instance of the Connection class.
	 *
	 * This should only be constructed by {@link Actor}.
	 *
	 * @protected
	 */
	constructor(
		actor: ActorInstance<S, CP, CS, V, I, DB, E, Q>,
		data: ConnDataInput<CP, CS>,
	) {
		this.#actor = actor;
		this.#stateManager = new StateManager(this, data);
	}

	/**
	 * Sends a raw message to the underlying connection.
	 */
	[CONN_SEND_MESSAGE_SYMBOL](message: CachedSerializer<any, any, any>) {
		if (this[CONN_DRIVER_SYMBOL]) {
			const driver = this[CONN_DRIVER_SYMBOL];

			if (driver.rivetKitProtocol) {
				driver.rivetKitProtocol.sendMessage(this.#actor, this, message);
			} else {
				this.#actor.rLog.warn({
					msg: "attempting to send RivetKit protocol message to connection that does not support it",
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
	send<K extends keyof E & string>(
		eventName: K,
		...args: InferEventArgs<InferSchemaMap<E>[K]>
	): void;
	send(
		eventName: keyof E extends never ? string : never,
		...args: unknown[]
	): void;
	send(eventName: string, ...args: unknown[]) {
		this.#assertConnected();
		if (!this[CONN_SPEAKS_RIVETKIT_SYMBOL]) {
			this.#actor.rLog.warn({
				msg: "cannot send messages to this connection type",
				connId: this.id,
				connType: this[CONN_DRIVER_SYMBOL]?.type,
			});
		}

		if (
			this.#actor.config.events !== undefined &&
			!hasSchemaConfigKey(this.#actor.config.events, eventName)
		) {
			this.#actor.rLog.warn({
				msg: "sending event not defined in actor events config",
				eventName,
				connId: this.id,
			});
		}

		const payload = args.length === 1 ? args[0] : args;
		const result = validateSchemaSync(
			this.#actor.config.events,
			eventName as keyof E & string,
			payload,
		);
		if (!result.success) {
			throw new EventPayloadInvalid(eventName, result.issues);
		}
		const eventArgs =
			args.length === 1
				? [result.data]
				: Array.isArray(result.data)
					? (result.data as unknown[])
					: args;
		this.#actor.emitTraceEvent("message.send", {
			"rivet.event.name": eventName,
			"rivet.conn.id": this.id,
		});
		const eventData = { name: eventName, args: eventArgs };
		this[CONN_SEND_MESSAGE_SYMBOL](
			new CachedSerializer(
				eventData,
				TO_CLIENT_VERSIONED,
				CLIENT_PROTOCOL_CURRENT_VERSION,
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

			try {
				await this.#actor.connectionManager.connDisconnected(this);
			} finally {
				this[CONN_DRIVER_SYMBOL] = undefined;
			}
		} else {
			this.#actor.rLog.warn({
				msg: "missing connection driver state for disconnect",
				conn: this.id,
			});
			this[CONN_DRIVER_SYMBOL] = undefined;
		}
	}
}
