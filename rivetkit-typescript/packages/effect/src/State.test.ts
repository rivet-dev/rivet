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

	it.effect(
		"getAndSet returns the previous value and commits the new value",
		() =>
			Effect.gen(function* () {
				const { s, cell } = yield* makeCellState(10);
				const previous = yield* State.getAndSet(s, 15);
				assert.strictEqual(previous, 10);
				assert.strictEqual(cell.value, 15);
			}),
	);

	it.effect("setAndGet returns the committed value", () =>
		Effect.gen(function* () {
			const { s, cell } = yield* makeCellState(10);
			const next = yield* State.setAndGet(s, 15);
			assert.strictEqual(next, 15);
			assert.strictEqual(cell.value, 15);
		}),
	);

	it.effect("update applies f over read/write", () =>
		Effect.gen(function* () {
			const { s, cell } = yield* makeCellState(10);
			yield* State.update(s, (n) => n + 5);
			assert.strictEqual(cell.value, 15);
		}),
	);

	it.effect("updateEffect applies an Effectful f over read/write", () =>
		Effect.gen(function* () {
			const { s, cell } = yield* makeCellState(10);
			yield* State.updateEffect(s, (n) => Effect.succeed(n + 5));
			assert.strictEqual(cell.value, 15);
		}),
	);

	it.effect(
		"getAndUpdate returns the previous value and commits the new value",
		() =>
			Effect.gen(function* () {
				const { s, cell } = yield* makeCellState(10);
				const previous = yield* State.getAndUpdate(s, (n) => n + 5);
				assert.strictEqual(previous, 10);
				assert.strictEqual(cell.value, 15);
			}),
	);

	it.effect(
		"getAndUpdateEffect returns the previous value and commits the new value",
		() =>
			Effect.gen(function* () {
				const { s, cell } = yield* makeCellState(10);
				const previous = yield* State.getAndUpdateEffect(s, (n) =>
					Effect.succeed(n + 5),
				);
				assert.strictEqual(previous, 10);
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

	it.effect("updateAndGetEffect returns the new value and commits it", () =>
		Effect.gen(function* () {
			const { s, cell } = yield* makeCellState(10);
			const next = yield* State.updateAndGetEffect(s, (n) =>
				Effect.succeed(n + 5),
			);
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

	it.effect("modifyEffect returns B and commits the new value", () =>
		Effect.gen(function* () {
			const { s, cell } = yield* makeCellState("a");
			const b = yield* State.modifyEffect(s, (str) =>
				Effect.succeed([str.length, `${str}b`] as const),
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

			yield* s[State.RuntimeTypeId].publishEffect(Effect.succeed(7));
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
					const sub = yield* PubSub.subscribe(
						s[State.RuntimeTypeId].pubsub,
					);
					assert.strictEqual(yield* PubSub.take(sub), 0);

					yield* s[State.RuntimeTypeId].publishEffect(
						Effect.succeed(1),
					);
					yield* s[State.RuntimeTypeId].publishEffect(
						Effect.succeed(2),
					);
					assert.strictEqual(yield* PubSub.take(sub), 1);
					assert.strictEqual(yield* PubSub.take(sub), 2);
				}),
			);
		}),
	);

	it.effect("shuts down the backing pubsub when its scope closes", () =>
		Effect.gen(function* () {
			const pubsub = yield* Effect.scoped(
				Effect.gen(function* () {
					const { s } = yield* makeCellState(0);
					return s[State.RuntimeTypeId].pubsub;
				}),
			);

			assert.isTrue(yield* PubSub.isShutdown(pubsub));
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

			yield* s.pipe(State.updateEffect((n) => Effect.succeed(n + 1)));
			assert.strictEqual(yield* State.get(s), 11);
		}),
	);

	it.effect("supports instance methods", () =>
		Effect.gen(function* () {
			const { s, cell } = yield* makeCellState(0);
			assert.strictEqual(yield* s.get, 0);

			yield* s.set(5);
			assert.strictEqual(cell.value, 5);

			const beforeSet = yield* s.getAndSet(6);
			assert.strictEqual(beforeSet, 5);
			assert.strictEqual(cell.value, 6);

			const afterSet = yield* s.setAndGet(7);
			assert.strictEqual(afterSet, 7);
			assert.strictEqual(cell.value, 7);

			yield* s.update((n) => n * 2);
			assert.strictEqual(yield* s.get, 14);

			yield* s.updateEffect((n) => Effect.succeed(n + 1));
			assert.strictEqual(yield* s.get, 15);

			const previousUpdate = yield* s.getAndUpdate((n) => n + 1);
			assert.strictEqual(previousUpdate, 15);
			assert.strictEqual(yield* s.get, 16);

			const previousEffectUpdate = yield* s.getAndUpdateEffect((n) =>
				Effect.succeed(n + 1),
			);
			assert.strictEqual(previousEffectUpdate, 16);
			assert.strictEqual(yield* s.get, 17);

			const next = yield* s.updateAndGet((n) => n + 1);
			assert.strictEqual(next, 18);

			const effectNext = yield* s.updateAndGetEffect((n) =>
				Effect.succeed(n + 1),
			);
			assert.strictEqual(effectNext, 19);

			const previous = yield* s.modify((n) => [n, n + 1] as const);
			assert.strictEqual(previous, 19);
			assert.strictEqual(yield* s.get, 20);

			const effectPrevious = yield* s.modifyEffect((n) =>
				Effect.succeed([n, n + 1] as const),
			);
			assert.strictEqual(effectPrevious, 20);
			assert.strictEqual(yield* s.get, 21);
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

	it.effect("Effectful update failure does not write", () =>
		Effect.gen(function* () {
			const { s, cell } = yield* makeCellState(0);
			const exit = yield* Effect.exit(
				State.updateEffect(s, () => Effect.fail("boom" as const)),
			);
			assert.isTrue(Exit.isFailure(exit));
			assert.strictEqual(cell.value, 0);
		}),
	);
});
