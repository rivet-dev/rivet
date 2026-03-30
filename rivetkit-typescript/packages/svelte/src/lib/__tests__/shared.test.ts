import "./runes-shim.js";
import { describe, expect, test } from "vitest";
import type { AnyActorRegistry } from "../index.js";
import {
	createReactiveConnection,
	createSharedRivetKit,
	withActorParams,
} from "../index.js";
import { createMockConnection } from "./helpers.js";

describe("shared helpers", () => {
	test("createSharedRivetKit reuses a single wrapper", () => {
		const client = { id: "client" } as never;
		let clientCalls = 0;

		const getRivet = createSharedRivetKit<AnyActorRegistry>(() => {
			clientCalls += 1;
			return client;
		});

		const a = getRivet();
		const b = getRivet();

		expect(a).toBe(b);
		expect(clientCalls).toBe(1);
	});

	test("withActorParams merges static and getter params", () => {
		let token = "first";

		const getOpts = withActorParams<AnyActorRegistry, never>(
			{
				name: "chat" as never,
				key: ["room-1"],
				params: { organizationId: "org-1" },
			},
			() => ({ token }),
		);

		expect(getOpts()).toEqual({
			name: "chat",
			key: ["room-1"],
			params: { organizationId: "org-1", token: "first" },
		});

		token = "second";

		expect(getOpts().params).toEqual({
			organizationId: "org-1",
			token: "second",
		});
	});

	test("withActorParams omits params when both inputs are undefined", () => {
		const getOpts = withActorParams<AnyActorRegistry, never>(
			{
				name: "chat" as never,
				key: ["room-1"],
			},
			() => undefined,
		);

		expect(getOpts()).toEqual({
			name: "chat",
			key: ["room-1"],
		});
	});

	test("createReactiveConnection reflects status, errors, and events", async () => {
		const mock = createMockConnection();
		const reactive = createReactiveConnection({
			connect: () => mock.connection,
		});

		expect(reactive.connStatus).toBe("idle");
		expect(reactive.isConnected).toBe(false);

		reactive.connect();
		mock.setStatus("connected");

		expect(reactive.connStatus).toBe("connected");
		expect(reactive.isConnected).toBe(true);

		let payload: string | null = null;
		const unsubscribe = reactive.onEvent("message", (value) => {
			payload = value as string;
		});

		mock.emit("message", "hello");
		expect(payload).toBe("hello");

		mock.emitError("boom");
		expect(reactive.error?.message).toBe("boom");

		unsubscribe();
		await reactive.dispose();

		expect(reactive.connStatus).toBe("disconnected");
		expect(reactive.connection).toBe(null);
	});
});
