import { Context, Schema } from "effect";
import type * as State from "./State";

const TypeId = "~@rivetkit/effect/ActorState";

/**
 * A typed, persistent state slot for one Rivet Actor. Yielded inside
 * the wake effect to obtain a `State` whose committed
 * changes are mirrored back to rivetkit's persisted state.
 *
 * State configuration (`schema` + `initial`) is server-only — it
 * describes the persisted shape and lives in implementation modules
 * (`live.ts`), not on the actor contract shared with clients.
 */
export interface ActorState<
	in out Name extends string,
	in out S extends Schema.Top,
> extends Context.Service<
		ActorState<Name, S>,
		State.State<S["Type"], Schema.SchemaError>
	> {
	readonly [TypeId]: typeof TypeId;
	readonly _tag: Name;
	readonly schema: S;
	readonly initialValue: () => S["Type"];
}

/**
 * Type-erased view of any `ActorState`.
 */
export interface Any {
	readonly [TypeId]: typeof TypeId;
	readonly _tag: string;
}

/**
 * Like `Any`, but with the prop fields (`schema`, `initialValue`) accessible.
 * Used by the runtime to seed `c.state` and provide the `State` under
 * the state's tag. The yielded `State` has no visible service requirement
 * because schema services are resolved against the actor runner context.
 */
export interface AnyWithProps
	extends Context.Service<any, State.State<any, Schema.SchemaError>> {
	readonly [TypeId]: typeof TypeId;
	readonly _tag: string;
	readonly schema: Schema.Top;
	readonly initialValue: () => unknown;
}

export const isActorState = (u: unknown): u is Any =>
	typeof u === "object" && u !== null && (u as any)[TypeId] === TypeId;

/**
 * Define a typed, persistent state slot for a Rivet Actor.
 *
 * `schema` is the persisted shape; `initialValue` produces the value used to
 * seed state on first wake. The returned value is itself a Context tag:
 * `yield* MyState` inside the wake effect resolves to a
 * `SubscriptionRef<S["Type"]>`.
 *
 * @example
 * ```ts
 * import { Schema } from "effect"
 * import { ActorState } from "@rivetkit/effect"
 *
 * const CounterState = ActorState.make("CounterState", {
 *   schema: Schema.Number,
 *   initialValue: () => 0,
 * })
 * ```
 */
export const make = <Name extends string, S extends Schema.Top>(
	name: Name,
	options: { readonly schema: S; readonly initialValue: () => S["Type"] },
): ActorState<Name, S> => {
	const tag = Context.Service<
		ActorState<Name, S>,
		State.State<S["Type"], Schema.SchemaError>
	>(`@rivetkit/effect/ActorState/${name}`) as ActorState<Name, S>;
	(tag as any)[TypeId] = TypeId;
	(tag as any)._tag = name;
	(tag as any).schema = options.schema;
	(tag as any).initialValue = options.initialValue;
	return tag;
};
