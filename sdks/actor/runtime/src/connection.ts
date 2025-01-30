import type { ProtocolFormat } from "@rivet-gg/actor-protocol/ws";
import type * as wsToClient from "@rivet-gg/actor-protocol/ws/to_client";
import * as cbor from "cbor-x";
import type { WSContext } from "hono/ws";
import type { AnyActor, ExtractActorConnState } from "./actor";
import * as errors from "./errors";
import { INSPECT_SYMBOL } from "./inspect";
import { logger } from "./log";
import { assertUnreachable } from "./utils";

export type IncomingWebSocketMessage = string | Blob | ArrayBufferLike;
export type OutgoingWebSocketMessage = string | ArrayBuffer | Uint8Array;

export type ConnectionId = number;

/**
 * Represents a client connection to an actor.
 *
 * Manages connection-specific data and controls the connection lifecycle.
 *
 * @see {@link https://rivet.gg/docs/connections|Connection Documentation}
 */
export class Connection<A extends AnyActor> {
	subscriptions: Set<string> = new Set<string>();

	#state: ExtractActorConnState<A> | undefined;
	#stateEnabled: boolean;

	/**
	 * If this connection has a socket attached so it can send messages to the client.
	 *
	 * @protected
	 */
	public get _isBidirectional() {
		return this._websocket !== undefined;
	}

	/**
	 * Unique identifier for the connection.
	 */
	public readonly id: ConnectionId;

	/**
	 * WebSocket context for managing the connection.
	 *
	 * @protected
	 */
	public _websocket: WSContext<WebSocket> | undefined;

	/**
	 * Protocol format used for message serialization and deserialization.
	 *
	 * @protected
	 */
	public _protocolFormat: ProtocolFormat;

	/**
	 * Gets the current state of the connection.
	 *
	 * Throws an error if the state is not enabled.
	 */
	public get state(): ExtractActorConnState<A> {
		this.#validateStateEnabled();
		if (!this.#state) throw new Error("state should exists");
		return this.#state;
	}

	/**
	 * Sets the state of the connection.
	 *
	 * Throws an error if the state is not enabled.
	 */
	public set state(value: ExtractActorConnState<A>) {
		this.#validateStateEnabled();
		this.#state = value;
	}

	/**
	 * Initializes a new instance of the Connection class.
	 *
	 * This should only be constructed by {@link Actor}.
	 *
	 * @param id - Unique identifier for the connection.
	 * @param websocket - WebSocket context for managing the connection.
	 * @param protocolFormat - Protocol format for message serialization and deserialization.
	 * @param state - Initial state of the connection.
	 * @param stateEnabled - Indicates if the state is enabled.
	 * @protected
	 */
	public constructor(
		id: ConnectionId,
		websocket: WSContext<WebSocket> | undefined,
		protocolFormat: ProtocolFormat,
		state: ExtractActorConnState<A> | undefined,
		stateEnabled: boolean,
	) {
		this.id = id;
		this._websocket = websocket;
		this._protocolFormat = protocolFormat;
		this.#state = state;
		this.#stateEnabled = stateEnabled;
	}

	#validateStateEnabled() {
		if (!this.#stateEnabled) {
			throw new errors.ConnectionStateNotEnabled();
		}
	}

	/**
	 * Parses incoming WebSocket messages based on the protocol format.
	 *
	 * @param data - The incoming WebSocket message.
	 * @returns The parsed message.
	 * @throws MalformedMessage if the message format is incorrect.
	 *
	 * @protected
	 */
	public async _parse(data: IncomingWebSocketMessage): Promise<unknown> {
		if (this._protocolFormat === "json") {
			if (typeof data !== "string") {
				logger().warn("received non-string for json parse");
				throw new errors.MalformedMessage();
			}
			return JSON.parse(data);
		}
		if (this._protocolFormat === "cbor") {
			if (data instanceof Blob) {
				const arrayBuffer = await data.arrayBuffer();
				return cbor.decode(new Uint8Array(arrayBuffer));
			}
			if (data instanceof ArrayBuffer) {
				return cbor.decode(new Uint8Array(data));
			}
			logger().warn("received non-binary type for cbor parse");
			throw new errors.MalformedMessage();
		}
		assertUnreachable(this._protocolFormat);
	}

	/**
	 * Serializes a value into a WebSocket message based on the protocol format.
	 *
	 * @param value - The value to serialize.
	 * @returns The serialized message.
	 *
	 * @protected
	 */
	public _serialize(value: unknown): OutgoingWebSocketMessage {
		if (this._protocolFormat === "json") {
			return JSON.stringify(value);
		}
		if (this._protocolFormat === "cbor") {
			// TODO: Remove this hack, but cbor-x can't handle anything extra in data structures
			const cleanValue = JSON.parse(JSON.stringify(value));
			return cbor.encode(cleanValue);
		}
		assertUnreachable(this._protocolFormat);
	}

	/**
	 * Sends a WebSocket message to the client.
	 *
	 * @param message - The message to send.
	 *
	 * @protected
	 */
	public _sendWebSocketMessage(message: OutgoingWebSocketMessage) {
		if (!this._websocket) {
			logger().warn(
				"attempting to send websocket message to connection without websocket",
			);
			return;
		}

		// TODO: Check WS state
		this._websocket.send(message);
	}

	/**
	 * Sends an event with arguments to the client.
	 *
	 * @param eventName - The name of the event.
	 * @param args - The arguments for the event.
	 * @see {@link https://rivet.gg/docs/events|Events Documentation}
	 */
	send(eventName: string, ...args: unknown[]) {
		this._sendWebSocketMessage(
			this._serialize({
				body: {
					ev: {
						n: eventName,
						a: args,
					},
				},
			} satisfies wsToClient.ToClient),
		);
	}

	/**
	 * Disconnects the client with an optional reason.
	 *
	 * @param reason - The reason for disconnection.
	 */
	disconnect(reason?: string) {
		this._websocket?.close(1000, reason);
	}

	/**
	 * Inspects the connection for debugging purposes.
	 * @internal
	 */
	[INSPECT_SYMBOL]() {
		return {
			id: this.id.toString(),
			subscriptions: [...this.subscriptions.values()],
			state: {
				enabled: this.#stateEnabled,
				native: this.#state,
			},
		};
	}
}
