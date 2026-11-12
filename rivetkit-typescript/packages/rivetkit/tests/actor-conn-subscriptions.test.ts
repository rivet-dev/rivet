import { describe, expect, test } from "vitest";
import { ActorConnRaw, CONNECT_SYMBOL } from "@/client/actor-conn";
import { ACTOR_CONNS_SYMBOL, type ClientRaw } from "@/client/client";
import type { UniversalWebSocket } from "@/common/websocket-interface";
import type { EngineControlClient } from "@/engine-client/driver";

class FakeWebSocket extends EventTarget {
	readyState = 1;
	sent: unknown[] = [];
	onListenersAttached?: () => void;

	override addEventListener(
		type: string,
		listener: EventListenerOrEventListenerObject | null,
	) {
		super.addEventListener(type, listener);
		// The connection registers its error listener last, so the socket is
		// fully wired once it arrives.
		if (type === "error") {
			queueMicrotask(() => this.onListenersAttached?.());
		}
	}

	send(data: string) {
		this.sent.push(JSON.parse(data));
	}

	close() {
		this.readyState = 3;
		this.dispatchEvent(
			Object.assign(new Event("close"), {
				code: 1000,
				reason: "Disposed",
				wasClean: true,
			}),
		);
	}

	receive(message: unknown) {
		this.dispatchEvent(
			Object.assign(new Event("message"), {
				data: JSON.stringify(message),
			}),
		);
	}

	subscriptions() {
		return this.sent
			.map((msg) => (msg as { body: { tag: string; val: unknown } }).body)
			.filter((body) => body.tag === "SubscriptionRequest")
			.map((body) => body.val);
	}
}

async function openConn() {
	const ws = new FakeWebSocket();
	const driver = {
		async openWebSocket() {
			return ws as unknown as UniversalWebSocket;
		},
	} as unknown as EngineControlClient;
	const client = {
		[ACTOR_CONNS_SYMBOL]: new Set(),
	} as unknown as ClientRaw;
	const conn = new ActorConnRaw(
		client,
		driver,
		undefined,
		undefined,
		"json",
		{ getForId: { name: "counter", actorId: "actor-1" } },
	);
	ws.onListenersAttached = () =>
		ws.receive({
			body: {
				tag: "Init",
				val: { actorId: "actor-1", connectionId: "c" },
			},
		});
	conn[CONNECT_SYMBOL]();
	await conn.ready;
	return { conn, ws };
}

describe("ActorConnRaw event subscriptions", () => {
	test("once() unsubscribes on the server after it fires", async () => {
		const { conn, ws } = await openConn();

		const received: unknown[] = [];
		conn.once("newCount", (count: unknown) => received.push(count));
		expect(ws.subscriptions()).toEqual([
			{ eventName: "newCount", subscribe: true },
		]);

		ws.receive({
			body: { tag: "Event", val: { name: "newCount", args: [5] } },
		});
		// Message handling awaits the parsed payload before dispatching.
		await new Promise((resolve) => setTimeout(resolve, 0));

		expect(received).toEqual([5]);
		expect(ws.subscriptions()).toEqual([
			{ eventName: "newCount", subscribe: true },
			{ eventName: "newCount", subscribe: false },
		]);

		await conn.dispose();
	});

	test("once() keeps the server subscription while other listeners remain", async () => {
		const { conn, ws } = await openConn();

		const received: unknown[] = [];
		conn.on("newCount", (count: unknown) => received.push(count));
		conn.once("newCount", (count: unknown) =>
			received.push(`once:${count}`),
		);

		ws.receive({
			body: { tag: "Event", val: { name: "newCount", args: [1] } },
		});
		// Message handling awaits the parsed payload before dispatching.
		await new Promise((resolve) => setTimeout(resolve, 0));

		expect(received).toEqual([1, "once:1"]);
		expect(ws.subscriptions()).toEqual([
			{ eventName: "newCount", subscribe: true },
		]);

		await conn.dispose();
	});

	test("a listener that resubscribes during dispatch stays subscribed", async () => {
		const { conn, ws } = await openConn();

		const received: unknown[] = [];
		const unsubscribe = conn.on("newCount", () => {
			unsubscribe();
			conn.on("newCount", (count: unknown) => received.push(count));
		});

		ws.receive({
			body: { tag: "Event", val: { name: "newCount", args: [1] } },
		});
		// Message handling awaits the parsed payload before dispatching.
		await new Promise((resolve) => setTimeout(resolve, 0));
		ws.receive({
			body: { tag: "Event", val: { name: "newCount", args: [2] } },
		});
		await new Promise((resolve) => setTimeout(resolve, 0));

		expect(received).toEqual([2]);
		expect(ws.subscriptions()).toEqual([
			{ eventName: "newCount", subscribe: true },
			{ eventName: "newCount", subscribe: false },
			{ eventName: "newCount", subscribe: true },
		]);

		await conn.dispose();
	});
});
