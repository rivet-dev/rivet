/**
 * `State` is a typed view over an actor's persisted state, plus a
 * subscribable stream of every change.
 *
 * Unlike a `Ref`, `State` has no in-memory cell — the persisted store
 * is the source of truth. Reads decode the live store on demand;
 * writes encode and overwrite it. A `PubSub<A>` backs {@link changes}
 * and is fed externally — the runtime publishes to it from rivetkit's
 * `onStateChange` callback so subscribers see every committed change,
 * including ones initiated outside the SDK.
 *
 * Read and write are Effect-typed so schemas with asynchronous
 * transforms (or service requirements) are supported. `update` and
 * `modify` serialize through a per-`State` semaphore so read/apply/
 * write triples are atomic across fibers; `set` shares the same lock
 * so all writes are linearized.
 *
 * The PubSub uses replay = 1, matching `SubscriptionRef`: a new
 * subscriber immediately sees the most recent value.
 */
import {
	Effect,
	Inspectable,
	identity,
	Pipeable,
	Predicate,
	PubSub,
	Semaphore,
	Scope,
	Stream,
	type Types,
} from "effect";
import { dual } from "effect/Function";

const TypeId = "~@rivetkit/effect/State";

/**
 * Internal access to `State`'s backing store and publish machinery.
 *
 * @internal
 */
export const RuntimeTypeId: unique symbol = Symbol.for(
	"~@rivetkit/effect/State/Runtime",
) as never;

/**
 * A view over a persisted state cell with a subscribable change stream.
 *
 * - `A` — the value type
 * - `E` — the read/write closures' failure type (e.g. a schema's
 *         `SchemaError` when read/write decode/encode against a schema)
 * - `R` — the read/write closures' service requirements
 */
export interface State<A, E = never, R = never>
	extends Variance<A, E, R>,
		Pipeable.Pipeable,
		Inspectable.Inspectable {
	/**
	 * Retrieves the persisted value.
	 */
	readonly get: Effect.Effect<A, E, R>;
	/**
	 * Retrieves the persisted value and sets a new value atomically.
	 */
	readonly getAndSet: (value: A) => Effect.Effect<A, E, R>;
	/**
	 * Retrieves the current value and updates it atomically with the result of
	 * applying a function.
	 */
	readonly getAndUpdate: (f: (a: A) => A) => Effect.Effect<A, E, R>;
	/**
	 * Retrieves the current value and updates it atomically with the result of
	 * applying an effectful function.
	 */
	readonly getAndUpdateEffect: <E2, R2>(
		f: (a: A) => Effect.Effect<A, E2, R2>,
	) => Effect.Effect<A, E | E2, R | R2>;
	/**
	 * Modifies atomically with a function that computes a return value and
	 * a new value.
	 */
	readonly modify: <B>(
		f: (a: A) => readonly [B, A],
	) => Effect.Effect<B, E, R>;
	/**
	 * Modifies atomically with an effectful function that computes a return value
	 * and a new value.
	 */
	readonly modifyEffect: <B, E2, R2>(
		f: (a: A) => Effect.Effect<readonly [B, A], E2, R2>,
	) => Effect.Effect<B, E | E2, R | R2>;
	/**
	 * Writes the persisted value.
	 */
	readonly set: (value: A) => Effect.Effect<void, E, R>;
	/**
	 * Sets the persisted value and returns the new value.
	 */
	readonly setAndGet: (value: A) => Effect.Effect<A, E, R>;
	/**
	 * Updates the persisted value with the result of applying a function.
	 */
	readonly update: (f: (a: A) => A) => Effect.Effect<void, E, R>;
	/**
	 * Updates the persisted value with the result of applying an effectful function.
	 */
	readonly updateEffect: <E2, R2>(
		f: (a: A) => Effect.Effect<A, E2, R2>,
	) => Effect.Effect<void, E | E2, R | R2>;
	/**
	 * Updates the persisted value with the result of applying an effectful function
	 * and returns the new value.
	 */
	readonly updateAndGet: (f: (a: A) => A) => Effect.Effect<A, E, R>;
	/**
	 * Updates the persisted value with the result of applying an effectful function
	 * and returns the new value.
	 */
	readonly updateAndGetEffect: <E2, R2>(
		f: (a: A) => Effect.Effect<A, E2, R2>,
	) => Effect.Effect<A, E | E2, R | R2>;
	/**
	 * Creates a stream that emits the current persisted value and all subsequent
	 * changes.
	 */
	readonly changes: Stream.Stream<A>;
	/**
	 * Internal access to `State`'s backing store and publish machinery.
	 *
	 * @internal
	 */
	readonly [RuntimeTypeId]: StateRuntime<A, E, R>;
}

/**
 * Runtime-only hooks for wiring actor state persistence and change
 * notifications.
 *
 * @internal
 */
export interface StateRuntime<A, E = never, R = never> {
	readonly publish: (value: A) => Effect.Effect<boolean>;
	readonly publishEffect: <E2, R2>(
		effect: Effect.Effect<A, E2, R2>,
	) => Effect.Effect<boolean, E2, R2>;
	readonly read: () => Effect.Effect<A, E, R>;
	readonly write: (value: A) => Effect.Effect<void, E, R>;
	readonly pubsub: PubSub.PubSub<A>;
}

export const isState = (u: unknown): u is State<unknown, unknown> =>
	Predicate.hasProperty(u, TypeId);

export interface Variance<A, E, R> {
	readonly [TypeId]: {
		readonly _A: Types.Invariant<A>;
		readonly _E: Types.Covariant<E>;
		readonly _R: Types.Covariant<R>;
	};
}

const Proto = {
	...Pipeable.Prototype,
	...Inspectable.BaseProto,
	[TypeId]: { _A: identity, _E: identity, _R: identity },
	toJSON(this: State<unknown, unknown, unknown>) {
		return { _id: "State" };
	},
};

/**
 * Creates a `State` from `read` and `write` closures over the
 * underlying store. The closures are responsible for any
 * encoding/decoding; `State` itself is schema-agnostic.
 *
 * The current value (per `read()`) is published to the pubsub on
 * construction so any subscription obtained later replays it.
 *
 * The backing PubSub is scoped and shuts down when the current
 * `Scope` closes.
 */
export const make = Effect.fnUntraced(function* <A, E, R>(
	read: () => Effect.Effect<A, E, R>,
	write: (value: A) => Effect.Effect<void, E, R>,
): Effect.fn.Return<State<A, E, R>, E, R | Scope.Scope> {
	const pubsub = yield* PubSub.unbounded<A>({ replay: 1 });
	yield* Effect.addFinalizer(() => PubSub.shutdown(pubsub));
	const initial = yield* read();
	yield* PubSub.publish(pubsub, initial);
	const self = Object.create(Proto);
	const semaphore = yield* Semaphore.make(1);
	self[RuntimeTypeId] = {
		read,
		write,
		pubsub,
		publish: (value: A) => PubSub.publish(pubsub, value),
		publishEffect: <E2, R2>(effect: Effect.Effect<A, E2, R2>) =>
			Semaphore.withPermit(
				semaphore,
				Effect.flatMap(effect, (value) =>
					PubSub.publish(pubsub, value),
				),
			),
	};
	self.get = Effect.suspend(() => self[RuntimeTypeId].read());
	self.set = (value: A) => Semaphore.withPermit(semaphore, write(value));
	self.getAndSet = (value: A) =>
		Semaphore.withPermit(
			semaphore,
			Effect.flatMap(read(), (previous) =>
				Effect.as(write(value), previous),
			),
		);
	self.setAndGet = (value: A) =>
		Semaphore.withPermit(semaphore, Effect.as(write(value), value));
	self.update = (f: (a: A) => A) =>
		Semaphore.withPermit(
			semaphore,
			Effect.flatMap(read(), (a) => write(f(a))),
		);
	self.updateEffect = <E2, R2>(f: (a: A) => Effect.Effect<A, E2, R2>) =>
		Semaphore.withPermit(
			semaphore,
			Effect.flatMap(read(), (a) => Effect.flatMap(f(a), write)),
		);
	self.getAndUpdate = (f: (a: A) => A) =>
		Semaphore.withPermit(
			semaphore,
			Effect.flatMap(read(), (previous) =>
				Effect.as(write(f(previous)), previous),
			),
		);
	self.getAndUpdateEffect = <E2, R2>(f: (a: A) => Effect.Effect<A, E2, R2>) =>
		Semaphore.withPermit(
			semaphore,
			Effect.flatMap(read(), (previous) =>
				Effect.flatMap(f(previous), (next) =>
					Effect.as(write(next), previous),
				),
			),
		);
	self.updateAndGet = (f: (a: A) => A) =>
		Semaphore.withPermit(
			semaphore,
			Effect.flatMap(read(), (a) => {
				const next = f(a);
				return Effect.as(write(next), next);
			}),
		);
	self.updateAndGetEffect = <E2, R2>(f: (a: A) => Effect.Effect<A, E2, R2>) =>
		Semaphore.withPermit(
			semaphore,
			Effect.flatMap(read(), (a) =>
				Effect.flatMap(f(a), (next) => Effect.as(write(next), next)),
			),
		);
	self.modify = <B>(f: (a: A) => readonly [B, A]) =>
		Semaphore.withPermit(
			semaphore,
			Effect.flatMap(read(), (a) => {
				const [b, next] = f(a);
				return Effect.as(write(next), b);
			}),
		);
	self.modifyEffect = <B, E2, R2>(
		f: (a: A) => Effect.Effect<readonly [B, A], E2, R2>,
	) =>
		Semaphore.withPermit(
			semaphore,
			Effect.flatMap(read(), (a) =>
				Effect.flatMap(f(a), ([b, next]) => Effect.as(write(next), b)),
			),
		);
	self.changes = Stream.fromPubSub(pubsub);
	return self;
});

export const get = <A, E, R>(self: State<A, E, R>): Effect.Effect<A, E, R> =>
	self.get;

export const getAndSet: {
	<A>(value: A): <E, R>(self: State<A, E, R>) => Effect.Effect<A, E, R>;
	<A, E, R>(self: State<A, E, R>, value: A): Effect.Effect<A, E, R>;
} = dual(
	2,
	<A, E, R>(self: State<A, E, R>, value: A): Effect.Effect<A, E, R> =>
		self.getAndSet(value),
);

export const getAndUpdate: {
	<A>(f: (a: A) => A): <E, R>(self: State<A, E, R>) => Effect.Effect<A, E, R>;
	<A, E, R>(self: State<A, E, R>, f: (a: A) => A): Effect.Effect<A, E, R>;
} = dual(
	2,
	<A, E, R>(self: State<A, E, R>, f: (a: A) => A): Effect.Effect<A, E, R> =>
		self.getAndUpdate(f),
);

export const getAndUpdateEffect: {
	<A, E2, R2>(
		f: (a: A) => Effect.Effect<A, E2, R2>,
	): <E, R>(self: State<A, E, R>) => Effect.Effect<A, E | E2, R | R2>;
	<A, E, R, E2, R2>(
		self: State<A, E, R>,
		f: (a: A) => Effect.Effect<A, E2, R2>,
	): Effect.Effect<A, E | E2, R | R2>;
} = dual(
	2,
	<A, E, R, E2, R2>(
		self: State<A, E, R>,
		f: (a: A) => Effect.Effect<A, E2, R2>,
	): Effect.Effect<A, E | E2, R | R2> => self.getAndUpdateEffect(f),
);

export const modify: {
	<A, B>(
		f: (a: A) => readonly [B, A],
	): <E, R>(self: State<A, E, R>) => Effect.Effect<B, E, R>;
	<A, E, R, B>(
		self: State<A, E, R>,
		f: (a: A) => readonly [B, A],
	): Effect.Effect<B, E, R>;
} = dual(
	2,
	<A, E, R, B>(
		self: State<A, E, R>,
		f: (a: A) => readonly [B, A],
	): Effect.Effect<B, E, R> => self.modify(f),
);

export const modifyEffect: {
	<A, B, E2, R2>(
		f: (a: A) => Effect.Effect<readonly [B, A], E2, R2>,
	): <E, R>(self: State<A, E, R>) => Effect.Effect<B, E | E2, R | R2>;
	<A, E, R, B, E2, R2>(
		self: State<A, E, R>,
		f: (a: A) => Effect.Effect<readonly [B, A], E2, R2>,
	): Effect.Effect<B, E | E2, R | R2>;
} = dual(
	2,
	<A, E, R, B, E2, R2>(
		self: State<A, E, R>,
		f: (a: A) => Effect.Effect<readonly [B, A], E2, R2>,
	): Effect.Effect<B, E | E2, R | R2> => self.modifyEffect(f),
);

export const set: {
	<A>(value: A): <E, R>(self: State<A, E, R>) => Effect.Effect<void, E, R>;
	<A, E, R>(self: State<A, E, R>, value: A): Effect.Effect<void, E, R>;
} = dual(
	2,
	<A, E, R>(self: State<A, E, R>, value: A): Effect.Effect<void, E, R> =>
		self.set(value),
);

export const setAndGet: {
	<A>(value: A): <E, R>(self: State<A, E, R>) => Effect.Effect<A, E, R>;
	<A, E, R>(self: State<A, E, R>, value: A): Effect.Effect<A, E, R>;
} = dual(
	2,
	<A, E, R>(self: State<A, E, R>, value: A): Effect.Effect<A, E, R> =>
		self.setAndGet(value),
);

export const update: {
	<A>(
		f: (a: A) => A,
	): <E, R>(self: State<A, E, R>) => Effect.Effect<void, E, R>;
	<A, E, R>(self: State<A, E, R>, f: (a: A) => A): Effect.Effect<void, E, R>;
} = dual(
	2,
	<A, E, R>(
		self: State<A, E, R>,
		f: (a: A) => A,
	): Effect.Effect<void, E, R> => self.update(f),
);

export const updateEffect: {
	<A, E2, R2>(
		f: (a: A) => Effect.Effect<A, E2, R2>,
	): <E, R>(self: State<A, E, R>) => Effect.Effect<void, E | E2, R | R2>;
	<A, E, R, E2, R2>(
		self: State<A, E, R>,
		f: (a: A) => Effect.Effect<A, E2, R2>,
	): Effect.Effect<void, E | E2, R | R2>;
} = dual(
	2,
	<A, E, R, E2, R2>(
		self: State<A, E, R>,
		f: (a: A) => Effect.Effect<A, E2, R2>,
	): Effect.Effect<void, E | E2, R | R2> => self.updateEffect(f),
);

export const updateAndGet: {
	<A>(f: (a: A) => A): <E, R>(self: State<A, E, R>) => Effect.Effect<A, E, R>;
	<A, E, R>(self: State<A, E, R>, f: (a: A) => A): Effect.Effect<A, E, R>;
} = dual(
	2,
	<A, E, R>(self: State<A, E, R>, f: (a: A) => A): Effect.Effect<A, E, R> =>
		self.updateAndGet(f),
);

export const updateAndGetEffect: {
	<A, E2, R2>(
		f: (a: A) => Effect.Effect<A, E2, R2>,
	): <E, R>(self: State<A, E, R>) => Effect.Effect<A, E | E2, R | R2>;
	<A, E, R, E2, R2>(
		self: State<A, E, R>,
		f: (a: A) => Effect.Effect<A, E2, R2>,
	): Effect.Effect<A, E | E2, R | R2>;
} = dual(
	2,
	<A, E, R, E2, R2>(
		self: State<A, E, R>,
		f: (a: A) => Effect.Effect<A, E2, R2>,
	): Effect.Effect<A, E | E2, R | R2> => self.updateAndGetEffect(f),
);

export const changes = <A, E, R>(self: State<A, E, R>): Stream.Stream<A> =>
	self.changes;
