import { assertExists } from "@std/assert/exists";
import * as cbor from "@std/cbor";
import type { WSContext } from "hono/ws";
import type { ProtocolFormat } from "../protocol/ws/mod.ts";
import type * as wsToClient from "../protocol/ws/to_client.ts";
import type { Actor, AnyActor, ExtractActorConnState } from "./actor.ts";
import * as errors from "./errors.ts";
import { logger } from "./log.ts";
import { assertUnreachable } from "./utils.ts";

export type IncomingWebSocketMessage = string | Blob | ArrayBufferLike;
export type OutgoingWebSocketMessage = string | ArrayBuffer | Uint8Array;

export type ConnectionId = number;

export class Connection<A extends AnyActor> {
	subscriptions: Set<string> = new Set<string>();

	#state: ExtractActorConnState<A> | undefined;
	#stateEnabled: boolean;

	public get state(): ExtractActorConnState<A> {
		this.#validateStateEnabled();
		assertExists(this.#state, "state should exist");
		return this.#state;
	}

	public set state(value: ExtractActorConnState<A>) {
		this.#validateStateEnabled();
		this.#state = value;
	}

	constructor(
		public readonly id: ConnectionId,
		public _websocket: WSContext<WebSocket>,
		public _protocolFormat: ProtocolFormat,
		state: ExtractActorConnState<A> | undefined,
		stateEnabled: boolean,
	) {
		this.#state = state;
		this.#stateEnabled = stateEnabled;
	}

	#validateStateEnabled() {
		if (!this.#stateEnabled) {
			throw new errors.ConnectionStateNotEnabled();
		}
	}

	public async _parse(data: IncomingWebSocketMessage): Promise<unknown> {
		if (this._protocolFormat === "json") {
			if (typeof data !== "string") {
				logger().warn("received non-string for json parse");
				throw new errors.MalformedMessage();
			}
			return JSON.parse(data);
		} else if (this._protocolFormat === "cbor") {
			if (data instanceof Blob) {
				return cbor.decodeCbor(await data.bytes());
			} else if (data instanceof ArrayBuffer) {
				return cbor.decodeCbor(new Uint8Array(data));
			} else {
				logger().warn("received non-binary type for cbor parse");
				throw new errors.MalformedMessage();
			}
		} else {
			assertUnreachable(this._protocolFormat);
		}
	}

	public _serialize(value: unknown): OutgoingWebSocketMessage {
		if (this._protocolFormat === "json") {
			return JSON.stringify(value);
		} else if (this._protocolFormat === "cbor") {
			return cbor.encodeCbor(value as cbor.CborType);
		} else {
			assertUnreachable(this._protocolFormat);
		}
	}

	public _sendWebSocketMessage(message: OutgoingWebSocketMessage) {
		// TODO: Queue message
		if (!this._websocket) return;

		// TODO: Check WS state
		this._websocket.send(message);
	}

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

	disconnect(reason?: string) {
		this._websocket.close(1000, reason);
	}
}
