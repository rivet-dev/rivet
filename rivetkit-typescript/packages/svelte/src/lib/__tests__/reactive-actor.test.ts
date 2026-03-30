import "./runes-shim.js";
import type { ActorConnStatus } from "rivetkit/client";
import { beforeEach, describe, expect, test, vi } from "vitest";

const frameworkMock = vi.hoisted(() => {
	type Listener = (...args: unknown[]) => void;
	type Subscriber = (value: { currentVal: MockActorState }) => void;
	type MockConnection = {
		id: string;
		ping: () => string;
		on: (eventName: string, handler: Listener) => () => void;
		emit: (eventName: string, ...args: unknown[]) => void;
	};
	type MockActorState = {
		connection: MockConnection;
		handle: { id: string };
		connStatus: ActorConnStatus;
		error: Error | null;
		hash: string;
	};

	const subscribers = new Set<Subscriber>();

	function createConnection(id: string): MockConnection {
		const listeners = new Map<string, Set<Listener>>();

		return {
			id,
			ping: () => `pong:${id}`,
			on(eventName: string, handler: Listener) {
				let eventListeners = listeners.get(eventName);
				if (!eventListeners) {
					eventListeners = new Set();
					listeners.set(eventName, eventListeners);
				}

				eventListeners.add(handler);
				return () => eventListeners?.delete(handler);
			},
			emit(eventName: string, ...args: unknown[]) {
				for (const listener of listeners.get(eventName) ?? []) {
					listener(...args);
				}
			},
		};
	}

	let currentState: MockActorState;

	const getOrCreateActor = vi.fn(() => ({
		mount: vi.fn(() => vi.fn()),
		state: {
			get state() {
				return currentState;
			},
			subscribe(callback: Subscriber) {
				subscribers.add(callback);
				return () => subscribers.delete(callback);
			},
		},
	}));

	function push(next: Partial<MockActorState>) {
		currentState = { ...currentState, ...next };
		for (const subscriber of subscribers) {
			subscriber({ currentVal: currentState });
		}
	}

	function reset() {
		subscribers.clear();
		currentState = {
			connection: createConnection("one"),
			handle: { id: "handle-one" },
			connStatus: "idle",
			error: null,
			hash: "hash-one",
		};
		getOrCreateActor.mockClear();
	}

	reset();

	return {
		getOrCreateActor,
		currentState: () => currentState,
		push,
		replaceConnection(id: string) {
			const connection = createConnection(id);
			push({
				connection,
				handle: { id: `handle-${id}` },
				hash: `hash-${id}`,
			});
			return connection;
		},
		reset,
	};
});

vi.mock("@rivetkit/framework-base", () => ({
	createRivetKit: vi.fn(() => ({
		getOrCreateActor: frameworkMock.getOrCreateActor,
	})),
}));

import { createRivetKitWithClient } from "../rivetkit.svelte.js";

describe("createReactiveActor", () => {
	beforeEach(() => {
		frameworkMock.reset();
	});

	test("caches proxied actor methods until the connection changes", () => {
		const rivet = createRivetKitWithClient({} as never);
		const actor = rivet.createReactiveActor({
			name: "chat" as never,
			key: ["room-1"],
		});

		const firstPing = actor.ping;
		const secondPing = actor.ping;

		expect(firstPing).toBe(secondPing);
		expect(firstPing()).toBe("pong:one");

		frameworkMock.replaceConnection("two");

		const thirdPing = actor.ping;
		expect(thirdPing).not.toBe(firstPing);
		expect(thirdPing()).toBe("pong:two");
	});

	test("preserves lastError and tracks hasEverConnected", () => {
		const rivet = createRivetKitWithClient({} as never);
		const actor = rivet.createReactiveActor({
			name: "chat" as never,
			key: ["room-1"],
		});

		expect(actor.lastError).toBe(null);
		expect(actor.hasEverConnected).toBe(false);

		frameworkMock.push({
			connStatus: "disconnected",
			error: new Error("boom"),
		});

		expect(actor.error?.message).toBe("boom");
		expect(actor.lastError?.message).toBe("boom");
		expect(actor.hasEverConnected).toBe(false);

		frameworkMock.push({
			connStatus: "connected",
			error: null,
		});

		expect(actor.isConnected).toBe(true);
		expect(actor.hasEverConnected).toBe(true);
		expect(actor.lastError?.message).toBe("boom");

		frameworkMock.push({
			connStatus: "disconnected",
			error: null,
		});

		expect(actor.error).toBe(null);
		expect(actor.lastError?.message).toBe("boom");
	});

	test("rebinds event listeners when the connection changes", () => {
		const rivet = createRivetKitWithClient({} as never);
		const actor = rivet.createReactiveActor({
			name: "chat" as never,
			key: ["room-1"],
		});

		const firstConnection = frameworkMock.currentState().connection;
		const received: string[] = [];

		actor.onEvent("message", (payload: unknown) => {
			received.push(String(payload));
		});

		firstConnection.emit("message", "one");
		expect(received).toEqual(["one"]);

		const secondConnection = frameworkMock.replaceConnection("two");

		firstConnection.emit("message", "stale");
		secondConnection.emit("message", "two");

		expect(received).toEqual(["one", "two"]);
	});
});
