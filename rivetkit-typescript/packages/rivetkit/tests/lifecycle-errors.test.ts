import { describe, expect, test } from "vitest";
import { ActorConnRaw, CONNECT_SYMBOL } from "@/client/actor-conn";
import { ACTOR_CONNS_SYMBOL, type ClientRaw } from "@/client/client";
import { ActorError } from "@/client/errors";
import {
	isRetryableLifecycleReconnectSignal,
	isRetryableLifecycleRequestError,
} from "@/client/lifecycle-errors";
import type { EngineControlClient } from "@/engine-client/driver";

function depotUnavailable() {
	return new ActorError(
		"depot",
		"unavailable",
		"The storage host could not finish the command (routing_unavailable); its outcome may be unknown.",
	);
}

describe("lifecycle error classification", () => {
	test("reconnects on depot.unavailable without replaying requests", () => {
		const error = depotUnavailable();

		expect(isRetryableLifecycleReconnectSignal(error)).toBe(true);
		expect(isRetryableLifecycleRequestError(error)).toBe(false);
	});

	test("connection retries the open after depot.unavailable", async () => {
		let attempts = 0;
		let reopened!: () => void;
		const reopen = new Promise<"reopened">((resolve) => {
			reopened = () => resolve("reopened");
		});
		const driver = {
			openWebSocket: async () => {
				attempts += 1;
				if (attempts === 1) throw depotUnavailable();
				reopened();
				return new Promise(() => {});
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
			{ getForId: { name: "test", actorId: "actor-id" } },
		);
		conn.onError(() => {});
		const idle = new Promise<"idle">((resolve) => {
			conn.onStatusChange((status) => {
				if (status === "idle") resolve("idle");
			});
		});

		conn[CONNECT_SYMBOL]();
		const outcome = await Promise.race([reopen, idle]);
		await conn.dispose();

		expect(outcome).toBe("reopened");
	});

	test("an action queued while the open fails is sent once, after the reopen", async () => {
		const sent: string[] = [];
		const listeners = new Map<string, (ev: { data: unknown }) => void>();
		const socket = {
			readyState: 1,
			addEventListener: (
				type: string,
				listener: (ev: { data: unknown }) => void,
			) => listeners.set(type, listener),
			send: (data: string) => sent.push(data),
			close: () => listeners.get("close")?.({ data: undefined }),
		};
		let attempts = 0;
		let reopened!: () => void;
		const reopen = new Promise<void>((resolve) => {
			reopened = resolve;
		});
		const driver = {
			openWebSocket: async () => {
				attempts += 1;
				if (attempts === 1) throw depotUnavailable();
				reopened();
				return socket;
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
			{ getForId: { name: "test", actorId: "actor-id" } },
		);
		conn.onError(() => {});
		let settled = false;

		conn[CONNECT_SYMBOL]();
		const action = conn.action({ name: "ping", args: [] }).finally(() => {
			settled = true;
		});
		action.catch(() => {});
		await reopen;
		expect(settled).toBe(false);
		expect(sent).toEqual([]);
		await new Promise((resolve) => setTimeout(resolve, 0));

		listeners.get("message")?.({
			data: JSON.stringify({
				body: {
					tag: "Init",
					val: { actorId: "actor-id", connectionId: "conn-id" },
				},
			}),
		});
		await new Promise((resolve) => setTimeout(resolve, 0));
		await conn.dispose();

		expect(
			sent.filter((message) => message.includes('"ping"')),
		).toHaveLength(1);
	});
});
