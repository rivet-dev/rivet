import { describe, expect, test, vi } from "vitest";
import { InlineWebSocketAdapter } from "./inline-websocket-adapter";

describe("InlineWebSocketAdapter", () => {
	test("buffers client messages until open completes", async () => {
		const events: string[] = [];
		const adapter = new InlineWebSocketAdapter({
			onOpen: () => {
				events.push("handler.open");
			},
			onMessage: (event: {
				data: string;
				rivetMessageIndex?: number;
			}) => {
				events.push(
					`handler.message:${event.data}:${event.rivetMessageIndex ?? "none"}`,
				);
			},
			onClose: () => {},
			onError: () => {},
		});

		adapter.clientWebSocket.addEventListener("open", () => {
			events.push("client.open");
		});
		adapter.actorWebSocket.addEventListener("open", () => {
			events.push("actor.open");
		});
		adapter.actorWebSocket.addEventListener(
			"message",
			(event: { data: string; rivetMessageIndex?: number }) => {
				events.push(
					`actor.message:${event.data}:${event.rivetMessageIndex ?? "none"}`,
				);
			},
		);

		adapter.dispatchClientMessageWithMetadata("hello", 7);
		expect(events).toEqual([]);

		await vi.waitFor(() => {
			expect(events).toEqual([
				"handler.open",
				"client.open",
				"actor.open",
				"handler.message:hello:7",
				"actor.message:hello:7",
			]);
		});
	});
});
