import { assertExists } from "@std/assert/exists";
import type { WSContext } from "hono/ws";
import type * as wsToClient from "../../protocol/src/ws/to_client.ts";
import type { Actor, AnyActor } from "./actor.ts";
import * as errors from "./errors.ts";

// biome-ignore lint/suspicious/noExplicitAny: Must be used for `extends`
type GetConnStateType<A> = A extends Actor<any, any, infer ConnState>
	? ConnState
	: never;

export class Connection<A extends AnyActor> {
	subscriptions = new Set<string>();

	#state: GetConnStateType<A> | undefined;
	#stateEnabled: boolean;

	public get state(): GetConnStateType<A> {
		this.#validateStateEnabled();
		assertExists(this.#state, "state should exist");
		return this.#state;
	}

	public set state(value: GetConnStateType<A>) {
		this.#validateStateEnabled();
		this.#state = value;
	}

	constructor(
		public readonly id: number,
		public _websocket: WSContext<WebSocket>,
		state: GetConnStateType<A> | undefined,
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

	_sendWebSocketMessage(message: string) {
		// TODO: Queue message
		if (!this._websocket) return;

		// TODO: Check WS state
		this._websocket.send(message);
	}

	send(eventName: string, ...args: unknown[]) {
		this._sendWebSocketMessage(
			JSON.stringify({
				body: {
					event: {
						name: eventName,
						args,
					},
				},
			} satisfies wsToClient.ToClient),
		);
	}

	disconnect(reason?: string) {
		this._websocket.close(1000, reason);
	}
}
