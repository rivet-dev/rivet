import { assert, describe, it } from "@effect/vitest";
import { State } from "@rivetkit/effect";
import { Effect, Exit, PubSub, Stream } from "effect";

// Helper: build a State backed by a plain mutable cell, with
// Effect-typed read/write closures. Mirrors how Registry wires
// `decodeUnknownEffect` / `encodeUnknownEffect` over `c.state`.
const makeCellState = <A>(initial: A) => {
	const cell = { value: initial };
	return State.make<A, never, never>(
		() => Effect.sync(() => cell.value),
		(v) =>
			Effect.sync(() => {
				cell.value = v;
			}),
	).pipe(Effect.map((s) => ({ s, cell })));
};

describe("State", () => {
	it.effect("get reflects the backing store", () =>
		Effect.gen(function* () {
			const { s, cell } = yield* makeCellState(42);
			assert.strictEqual(yield* State.get(s), 42);

			cell.value = 100;
			assert.strictEqual(yield* State.get(s), 100);
		}),
	);

	it.effect("set writes through to the backing store", () =>
		Effect.gen(function* () {
			const { s, cell } = yield* makeCellState(0);
			yield* State.set(s, 7);
			assert.strictEqual(cell.value, 7);
			assert.strictEqual(yield* State.get(s), 7);
		}),
	);

	it.effect("update applies f over read/write", () =>
		Effect.gen(function* () {
			const { s, cell } = yield* makeCellState(10);
			yield* State.update(s, (n) => n + 5);
			assert.strictEqual(cell.value, 15);
		}),
	);

	it.effect("updateAndGet returns the new value and commits it", () =>
		Effect.gen(function* () {
			const { s, cell } = yield* makeCellState(10);
			const next = yield* State.updateAndGet(s, (n) => n + 5);
			assert.strictEqual(next, 15);
			assert.strictEqual(cell.value, 15);
		}),
	);

	it.effect("modify returns B and commits the new value", () =>
		Effect.gen(function* () {
			const { s, cell } = yield* makeCellState("a");
			const b = yield* State.modify(
				s,
				(str) => [str.length, `${str}b`] as const,
			);
			assert.strictEqual(b, 1);
			assert.strictEqual(cell.value, "ab");
		}),
	);

	it.effect(
		"update is atomic across concurrent fibers (no lost updates)",
		() =>
			Effect.gen(function* () {
				const { s, cell } = yield* makeCellState(0);
				yield* Effect.all(
					Array.from({ length: 100 }, () =>
						State.update(s, (n) => n + 1),
					),
					{ concurrency: "unbounded" },
				);
				assert.strictEqual(cell.value, 100);
			}),
	);

	it.effect("changes replays the most recent published value", () =>
		Effect.gen(function* () {
			const { s } = yield* makeCellState(0);
			const initial = yield* State.changes(s).pipe(
				Stream.take(1),
				Stream.runCollect,
			);
			assert.deepStrictEqual(initial, [0]);

			State.publishUnsafe(s, 7);
			const later = yield* State.changes(s).pipe(
				Stream.take(1),
				Stream.runCollect,
			);
			assert.deepStrictEqual(later, [7]);
		}),
	);

	it.effect("publish pushes values to live subscribers", () =>
		Effect.gen(function* () {
			const { s } = yield* makeCellState(0);
			yield* Effect.scoped(
				Effect.gen(function* () {
					const sub = yield* PubSub.subscribe(s.pubsub);
					assert.strictEqual(yield* PubSub.take(sub), 0);

					yield* State.publish(s, 1);
					yield* State.publish(s, 2);
					assert.strictEqual(yield* PubSub.take(sub), 1);
					assert.strictEqual(yield* PubSub.take(sub), 2);
				}),
			);
		}),
	);

	it.effect("set does NOT auto-publish — the runtime does", () =>
		Effect.gen(function* () {
			const { s } = yield* makeCellState(0);
			yield* State.set(s, 99);
			// replay should still hold the initial 0, not 99
			const latest = yield* State.changes(s).pipe(
				Stream.take(1),
				Stream.runCollect,
			);
			assert.deepStrictEqual(latest, [0]);
		}),
	);

	it.effect("isState discriminates", () =>
		Effect.gen(function* () {
			const { s } = yield* makeCellState(0);
			assert.isTrue(State.isState(s));
			assert.isFalse(State.isState({}));
			assert.isFalse(State.isState(null));
			assert.isFalse(State.isState(42));
		}),
	);

	it.effect("supports .pipe()", () =>
		Effect.gen(function* () {
			const { s } = yield* makeCellState(0);
			yield* s.pipe(State.set(5));
			assert.strictEqual(yield* State.get(s), 5);

			yield* s.pipe(State.update((n) => n * 2));
			assert.strictEqual(yield* State.get(s), 10);
		}),
	);

	it.effect("read failure propagates through get", () =>
		Effect.gen(function* () {
			const reads = { count: 0 };
			// Construction reads once to seed the pubsub; subsequent reads
			// fail. Mirrors a schema mismatch on persisted state.
			const s = yield* State.make<number, "boom", never>(
				() =>
					Effect.suspend(() => {
						reads.count++;
						if (reads.count === 1) return Effect.succeed(0);
						return Effect.fail("boom" as const);
					}),
				() => Effect.void,
			);
			const exit = yield* Effect.exit(State.get(s));
			assert.isTrue(Exit.isFailure(exit));
		}),
	);

	it.effect("write failure propagates through set", () =>
		Effect.gen(function* () {
			const s = yield* State.make<number, "boom", never>(
				() => Effect.succeed(0),
				() => Effect.fail("boom" as const),
			);
			const exit = yield* Effect.exit(State.set(s, 1));
			assert.isTrue(Exit.isFailure(exit));
		}),
	);
});
