import { type Context, Effect, Schema } from "effect";
import * as Rivetkit from "rivetkit";
import type { Actor, Fork } from "../Actor.ts";
import { runInstanceEffect } from "./ActionDispatcher.ts";
import { makeActorLogAnnotations } from "./logging.ts";

type RivetkitContext = {
	readonly actorId: string;
	readonly name: string;
	readonly key: Rivetkit.ActorKey;
};

const makeDecodeCreationInput = (
	actor: Pick<Actor<string, any, any, any, any>, "name" | "input">,
) => {
	if (actor.input === undefined) return undefined;

	const decodeInput = Schema.decodeUnknownEffect(
		Schema.toCodecJson(actor.input),
	);
	return (input: unknown) =>
		decodeInput(input).pipe(
			Effect.tapError((schemaError) =>
				Effect.logDebug(
					`Invalid input for actor ${actor.name}`,
					schemaError,
				),
			),
			Effect.mapError(
				() =>
					new Rivetkit.RivetError(
						"request",
						"invalid",
						`Invalid input for actor ${actor.name}`,
					),
			),
		) as Effect.Effect<unknown, Rivetkit.RivetError>;
};

/**
 * Builds the RivetKit callbacks for the `toLayer` hooks that RivetKit runs
 * before `onWake`. No actor instance exists then, so each hook runs with the
 * layer's services plus the context `makeContext` builds from the RivetKit
 * context.
 */
export const makeRivetkitCallbacks = Effect.fnUntraced(function* <
	C extends RivetkitContext,
>({
	actor,
	makeContext,
	onCreate,
	onMigrate,
	onFork,
}: {
	readonly actor: Pick<Actor<string, any, any, any, any>, "name" | "input">;
	readonly makeContext: (c: C) => Effect.Effect<Context.Context<any>>;
	readonly onCreate:
		| ((input: unknown) => Effect.Effect<void, never, any>)
		| undefined;
	readonly onMigrate:
		| ((options: {
				readonly isNew: boolean;
		  }) => Effect.Effect<void, never, any>)
		| undefined;
	readonly onFork:
		| ((fork: Fork<Schema.Top>) => Effect.Effect<void, never, any>)
		| undefined;
}) {
	const runFork = Effect.runForkWith(yield* Effect.context<any>());
	const decodeCreationInput = makeDecodeCreationInput(actor);

	const run = (
		c: C,
		hookName: string,
		hook: Effect.Effect<void, unknown, any>,
	) =>
		runInstanceEffect(
			runFork,
			makeContext(c).pipe(
				Effect.flatMap((context) =>
					hook.pipe(
						Effect.withSpan(`${actor.name}/${hookName}`, {
							kind: "server",
						}),
						Effect.provideContext(context),
					),
				),
				Effect.annotateLogs(makeActorLogAnnotations(c)),
			),
		);

	return {
		/** An actor that declares `input` rejects input that does not decode, also without an `onCreate`. */
		onCreate:
			decodeCreationInput === undefined && onCreate === undefined
				? undefined
				: (c: C, input: unknown) =>
						run(
							c,
							"onCreate",
							(decodeCreationInput === undefined
								? Effect.succeed(undefined)
								: decodeCreationInput(input)
							).pipe(
								Effect.flatMap((decodedInput) =>
									onCreate === undefined
										? Effect.void
										: onCreate(decodedInput),
								),
							),
						),
		onMigrate:
			onMigrate === undefined
				? undefined
				: (c: C, isNew: boolean) =>
						run(c, "onMigrate", onMigrate({ isNew })),
		/** Like `onCreate`, an actor that declares `input` rejects fork input that does not decode. */
		onFork:
			decodeCreationInput === undefined && onFork === undefined
				? undefined
				: (
						c: C,
						fork: Omit<Fork, "input"> & { readonly input: unknown },
					) =>
						run(
							c,
							"onFork",
							(decodeCreationInput === undefined
								? Effect.succeed(undefined)
								: decodeCreationInput(fork.input)
							).pipe(
								Effect.flatMap((decodedInput) =>
									onFork === undefined
										? Effect.void
										: onFork({
												...fork,
												input: decodedInput,
											}),
								),
							),
						),
	};
});
