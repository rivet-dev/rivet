import { describe, expect, test, vi } from "vitest";
import {
	handleInboundHibernatableWebSocketMessage,
	HibernatableWebSocketAckState,
	HIBERNATABLE_WEBSOCKET_ACK_DEADLINE,
	HIBERNATABLE_WEBSOCKET_BUFFERED_MESSAGE_SIZE_THRESHOLD,
} from "@/actor/conn/hibernatable-websocket-ack-state";

describe("hibernatable websocket ack state", () => {
	test("schedules persistence for indexed messages without extra actor writes", () => {
		const ackState = new HibernatableWebSocketAckState();
		ackState.createConnEntry("conn-1", 0);
		const saveState = vi.fn();
		const hibernatable = {
			serverMessageIndex: 0,
		};

		handleInboundHibernatableWebSocketMessage({
			connId: "conn-1",
			hibernatable,
			messageLength: 32,
			rivetMessageIndex: 1,
			ackState,
			saveState,
		});

		expect(hibernatable.serverMessageIndex).toBe(1);
		expect(saveState).toHaveBeenCalledWith({
			maxWait: HIBERNATABLE_WEBSOCKET_ACK_DEADLINE,
		});
	});

	test("forces immediate persistence when buffered size reaches the threshold", () => {
		const ackState = new HibernatableWebSocketAckState();
		ackState.createConnEntry("conn-1", 0);
		const saveState = vi.fn();
		const hibernatable = {
			serverMessageIndex: 0,
		};

		handleInboundHibernatableWebSocketMessage({
			connId: "conn-1",
			hibernatable,
			messageLength:
				HIBERNATABLE_WEBSOCKET_BUFFERED_MESSAGE_SIZE_THRESHOLD,
			rivetMessageIndex: 1,
			ackState,
			saveState,
		});

		expect(hibernatable.serverMessageIndex).toBe(1);
		expect(saveState).toHaveBeenCalledWith({
			immediate: true,
		});
	});

	test("acks the persisted message index only after persist completes", () => {
		const ackState = new HibernatableWebSocketAckState();
		ackState.createConnEntry("conn-1", 0);
		const saveState = vi.fn();
		const hibernatable = {
			serverMessageIndex: 0,
		};

		handleInboundHibernatableWebSocketMessage({
			connId: "conn-1",
			hibernatable,
			messageLength: 32,
			rivetMessageIndex: 1,
			ackState,
			saveState,
		});

		expect(ackState.consumeAck("conn-1")).toBeUndefined();
		expect(
			ackState.onBeforePersist("conn-1", hibernatable.serverMessageIndex),
		).toBe(true);
		expect(ackState.consumeAck("conn-1")).toBe(1);
		expect(ackState.consumeAck("conn-1")).toBeUndefined();
	});
});
