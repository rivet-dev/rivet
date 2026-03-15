import { Effect } from "effect";
import { describe, expect, it, vi } from "@effect/vitest";
import * as Queue from "../src/queue.ts";

const createContext = (nextFn: (...args: any[]) => Promise<any>) => ({
	queue: { next: nextFn },
});

describe("@rivetkit/effect queue helpers", () => {
	it.effect("returns undefined when queue has no next message", () =>
		Effect.gen(function* () {
			const ctx = createContext(vi.fn(async () => undefined));
			const result = yield* Queue.next(ctx as any, "jobs");
			expect(result).toBeUndefined();
		}),
	);

	it.effect("returns a QueueMessage when one is available", () =>
		Effect.gen(function* () {
			const message = { id: 1n, name: "jobs", body: { task: "test" }, createdAt: Date.now() };
			const ctx = createContext(vi.fn(async () => message));
			const result = yield* Queue.next(ctx as any, "jobs");
			expect(result).toEqual(message);
		}),
	);

	it.effect("passes options through to queue.next", () =>
		Effect.gen(function* () {
			const nextFn = vi.fn(async () => undefined);
			const ctx = createContext(nextFn);
			yield* Queue.next(ctx as any, "jobs", { timeout: 5000, count: 3 });
			expect(nextFn).toHaveBeenCalledWith("jobs", { timeout: 5000, count: 3 });
		}),
	);

	it.effect("nextMultiple returns undefined when no messages available", () =>
		Effect.gen(function* () {
			const ctx = createContext(vi.fn(async () => undefined));
			const result = yield* Queue.nextMultiple(ctx as any, ["jobs", "audit"]);
			expect(result).toBeUndefined();
		}),
	);

	it.effect("nextMultiple returns array of messages", () =>
		Effect.gen(function* () {
			const messages = [
				{ id: 1n, name: "jobs", body: {}, createdAt: Date.now() },
				{ id: 2n, name: "audit", body: {}, createdAt: Date.now() },
			];
			const ctx = createContext(vi.fn(async () => messages));
			const result = yield* Queue.nextMultiple(ctx as any, ["jobs", "audit"]);
			expect(result).toEqual(messages);
		}),
	);

	it.effect("nextMultiple passes names array to queue.next", () =>
		Effect.gen(function* () {
			const nextFn = vi.fn(async () => undefined);
			const ctx = createContext(nextFn);
			yield* Queue.nextMultiple(ctx as any, ["jobs", "audit"], { timeout: 1000 });
			expect(nextFn).toHaveBeenCalledWith(["jobs", "audit"], { timeout: 1000 });
		}),
	);
});
