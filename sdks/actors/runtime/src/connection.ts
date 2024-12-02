import { WSContext } from "hono/ws";
import * as wsToClient from "../../protocol/src/ws/to_client.ts";

export class Connection<Data> {
	// TODO: Add assert when trying to access data (like in state)

	// TODO: Expose request data

	// TODO: Make these private

	// TODO: Make private
	websocket?: WSContext<WebSocket>;

	// TODO: Probably needs a safe getter/setter
	data?: Data;

	subscriptions = new Set<string>();

	sendWebSocketMessage(message: string) {
		// TODO: Queue message
		if (!this.websocket) return;

		// TODO: Check WS state
		this.websocket?.send(message);
	}

	send(eventName: string, ...args: unknown[]) {
		this.sendWebSocketMessage(JSON.stringify({
			body: {
				event: {
					name: eventName,
					args,
				}
			}
		} satisfies wsToClient.ToClient))
	}
}

