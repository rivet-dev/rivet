import { Effect, Exit } from "effect";
import { describe, expect, it, vi } from "@effect/vitest";
import * as Action from "../src/action.ts";
import * as Actor from "../src/actor.ts";
import { OnCreate, OnStateChange } from "../src/lifecycle.ts";

const createContext = () => {
	const waitUntil = vi.fn((promise: Promise<unknown>) => {
		void promise.catch(() => undefined);
	});

	return {
		state: { count: 0 },
		vars: {},
		log: {
			info: vi.fn(),
			warn: vi.fn(),
			error: vi.fn(),
			debug: vi.fn(),
		},
		actorId: "actor-1",
		name: "message",
		key: ["key-1"],
		region: "test",
		schedule: null,
		conns: [],
		client: vi.fn(() => ({})),
		db: {},
		kv: {},
		waitUntil,
		abortSignal: new AbortController().signal,
		sleep: vi.fn(),
		destroy: vi.fn(),
		saveState: vi.fn(async () => undefined),
		broadcast: vi.fn(),
		conn: { state: null },
	};
};

describe("@rivetkit/effect wrappers", () => {
	it("Action.effect provides actor context and resolves async result", async () => {
		const ctx = createContext();
		const add = Action.effect(function* (c: any, amount: number) {
			yield* Action.updateState(c, (s: { count: number }) => {
				s.count += amount;
			});
			return yield* Action.state(c);
		});

		const state = await add(ctx as any, 3);
		expect(state).toEqual({ count: 3 });
	});

	it.effect("saveState maps promise rejection into StatePersistenceError", () =>
		Effect.gen(function* () {
			const ctx = createContext();
			ctx.saveState = vi.fn(async () => {
				throw new Error("write failure");
			});
			const exit = yield* Effect.exit(Actor.saveState(ctx as any, { debounce: 0 } as any));
			expect(Exit.isFailure(exit)).toBe(true);
			if (Exit.isFailure(exit)) {
				const failure = yield* Effect.succeed(exit.cause.toString());
				expect(failure).toContain("StatePersistenceError");
			}
		}),
	);

	it("OnStateChange logs effect failures", async () => {
		const ctx = createContext();
		const onStateChange = OnStateChange.effect(function* () {
			yield* Effect.fail("state-sync-failed");
		});

		onStateChange(ctx as any, { count: 1 });
		await new Promise((resolve) => setTimeout(resolve, 0));

		expect(ctx.log.error).toHaveBeenCalledWith(
			expect.objectContaining({ msg: "onStateChange effect failed" }),
		);
	});

	it("OnCreate.effect runs async lifecycle and returns result", async () => {
		const ctx = createContext();
		const onCreate = OnCreate.effect(function* (c: any, input: { name: string }) {
			yield* Actor.updateState(c, (s: { count: number }) => {
				s.count = 42;
			});
			return input.name;
		});

		const result = await onCreate(ctx as any, { name: "test-actor" });
		expect(result).toBe("test-actor");
		expect(ctx.state.count).toBe(42);
	});
});
