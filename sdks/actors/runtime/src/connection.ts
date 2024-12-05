import { WSContext } from "hono/ws";
import type { Actor } from "./actor.ts";
import * as wsToClient from "../../protocol/src/ws/to_client.ts";

type GetConnStateType<A> = A extends Actor<any, any, infer ConnState> ? ConnState : never;

export class Connection<A extends Actor<any, any, any>> {
	subscriptions = new Set<string>();

	#state: GetConnStateType<A>;
	#stateEnabled: boolean;

	public get state(): GetConnStateType<A> {
		this.#validateStateEnabled();
		return this.#state;
	}

	public set state(value: GetConnStateType<A>) {
		this.#validateStateEnabled();
		this.#state = value;
	}

	constructor(
		public readonly id: number,
		public _websocket: WSContext<WebSocket>,
		state: GetConnStateType<A>,
		stateEnabled: boolean,
	) {
		this.#state = state;
		this.#stateEnabled = stateEnabled;
	}

	#validateStateEnabled() {
		if (!this.#stateEnabled) {
			throw new Error(
				"Connection state not enabled. Must implement prepareConnection to use connection state.",
			);
		}
	}

	_sendWebSocketMessage(message: string) {
		// TODO: Queue message
		if (!this._websocket) return;

		// TODO: Check WS state
		this._websocket.send(message);
	}

	send(eventName: string, ...args: unknown[]) {
		this._sendWebSocketMessage(JSON.stringify(
			{
				body: {
					event: {
						name: eventName,
						args,
					},
				},
			} satisfies wsToClient.ToClient,
		));
	}

	disconnect(reason?: string) {
		this._websocket.close(1000, reason);
	}
}
