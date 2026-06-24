import { Effect, type Fiber, Schema } from "effect";
import * as State from "../State.ts";
import type * as StateOptions from "./StateOptions.ts";

export type ActorState<StateDefinition extends StateOptions.Any> = State.State<
	StateOptions.Decoded<StateDefinition>,
	Schema.SchemaError,
	StateOptions.Services<StateDefinition>
>;

type StateInstance<StateDefinition extends StateOptions.Any> = {
	readonly runFork: <A, E>(
		effect: Effect.Effect<A, E, any>,
		options?: Effect.RunOptions,
	) => Fiber.Fiber<A, E>;
	readonly state?: ActorState<StateDefinition>;
};

export type Adapter<StateDefinition extends StateOptions.Any> = {
	readonly makeStateView: (c: {
		state: StateOptions.Encoded<StateDefinition>;
	}) => Effect.Effect<ActorState<StateDefinition>, never, any>;
	readonly createInitialState: () => Promise<
		StateOptions.Encoded<StateDefinition>
	>;
	readonly publishChange: (
		instance: StateInstance<StateDefinition>,
		newState: unknown,
	) => void;
};

export const make = Effect.fnUntraced(function* <
	StateDefinition extends StateOptions.Any,
>(
	stateOptions: StateDefinition,
): Effect.fn.Return<Adapter<StateDefinition>, never, any> {
	const services = yield* Effect.context<any>();

	const stateCodec = {
		decodeUnknown: Schema.decodeUnknownEffect(
			Schema.toCodecJson(stateOptions.schema),
		),
		encode: Schema.encodeEffect(Schema.toCodecJson(stateOptions.schema)),
	};

	return {
		makeStateView: (c) =>
			State.make(
				() => stateCodec.decodeUnknown(c.state),
				(next) =>
					stateCodec.encode(next).pipe(
						Effect.tap((encoded) =>
							Effect.sync(() => {
								c.state = encoded;
							}),
						),
						Effect.asVoid,
					),
			).pipe(
				Effect.orDie,
				Effect.map((state) => state as ActorState<StateDefinition>),
			),
		createInitialState: () =>
			Effect.runPromiseWith(services)(
				stateCodec
					.encode(stateOptions.initialValue())
					.pipe(Effect.orDie),
			),
		publishChange: (instance, newState) => {
			instance.runFork(
				Effect.gen(function* () {
					const state = yield* Effect.fromNullishOr(
						instance.state,
					).pipe(Effect.orDie);
					yield* state[State.RuntimeTypeId].publishEffect(
						stateCodec.decodeUnknown(newState).pipe(Effect.orDie),
					);
				}),
			);
		},
	};
});
