import { Context, Effect, Layer, Record, Result, Schema } from "effect";
import * as RivetkitClient from "rivetkit/client";
import * as RivetkitErrors from "rivetkit/errors";
import type * as Action from "./Action";
import type * as Actor from "./Actor";
import * as ActionErrorEnvelope from "./internal/ActionErrorEnvelope";
import { rpcSystem, type TraceMeta } from "./internal/tracing";
import * as RivetError from "./RivetError";

const TypeId = "~@rivetkit/effect/Client";

/**
 * Connection options for the Rivet Engine client transport. Mirrors
 * the `(endpoint, token, namespace)` subset of rivetkit's
 * `ClientConfigInput`.
 */
export type Options = Pick<
	RivetkitClient.ClientConfigInput,
	"endpoint" | "token" | "namespace"
>;

/**
 * Per-call metadata envelope shipped as `args[1]` alongside the encoded
 * payload. The SDK currently uses it for trace propagation (`trace`),
 * but it's intentionally extensible so future cross-cutting concerns —
 * idempotency keys, deadlines, custom headers — can land as additional
 * optional fields without changing the wire shape.
 */
export interface ActionMeta {
	readonly trace?: TraceMeta;
}

export interface Client {
	readonly [TypeId]: typeof TypeId;

	readonly makeActorAccessor: <Actions extends Action.AnyWithProps>(
		actor: Actor.Actor<string, Actions>,
	) => Actor.Accessor<Actions>;
}

export const Client: Context.Service<Client, Client> = Context.Service<Client>(
	"@rivetkit/effect/Client",
);

export const make = Effect.fnUntraced(function* (options: Options = {}) {
	const rivetkitClient = yield* Effect.acquireRelease(
		Effect.sync(() => RivetkitClient.createClient(options)),
		(c) => Effect.promise(() => c.dispose()),
	);

	return Client.of({
		[TypeId]: TypeId,
		makeActorAccessor: (actor) => ({
			getOrCreate: (key) => {
				const rivetkitActorHandle = rivetkitClient.getOrCreate(
					actor.name,
					key,
				);

				return Record.fromIterableWith(actor.actions, (action) => {
					const encodePayload = Schema.encodeEffect(
						Schema.toCodecJson(action.payloadSchema),
					);
					const decodeSuccess = Schema.decodeUnknownEffect(
						Schema.toCodecJson(action.successSchema),
					);
					const classifyRivetkitActionFailure =
						makeRivetkitActionFailureClassifier(action.errorSchema);

					const rpcMethod = `${actor.name}/${action._tag}`;

					return [
						action._tag,
						Effect.fn(rpcMethod, {
							kind: "client",
							attributes: {
								"rpc.system.name": rpcSystem,
								"rpc.method": rpcMethod,
							},
						})(function* (payload: unknown) {
							const span = yield* Effect.currentSpan;
							const meta: ActionMeta = {
								trace: {
									traceId: span.traceId,
									spanId: span.spanId,
									sampled: span.sampled,
								},
							};
							const encodedPayload = yield* encodePayload(
								payload,
							).pipe(
								Effect.mapError(
									(cause) =>
										new RivetError.RivetError({
											reason: new RivetError.InvalidEncoding(
												{
													cause: new RivetkitErrors.RivetError(
														"encoding",
														"invalid",
														"Could not encode action payload",
														{
															public: true,
															metadata: cause,
														},
													),
												},
											),
										}),
								),
							);

							const encodedSuccess = yield* Effect.tryPromise(
								(abortSignal) =>
									rivetkitActorHandle.action({
										name: action._tag,
										args: [encodedPayload, meta],
										signal: abortSignal,
									}),
							).pipe(
								Effect.catch((unknownError) =>
									classifyRivetkitActionFailure(
										unknownError.cause,
									).pipe(Effect.flatMap(Effect.fail)),
								),
							);

							return yield* decodeSuccess(encodedSuccess).pipe(
								Effect.orDie,
							);
						}),
					];
				}) as Actor.Handle<(typeof actor.actions)[number]>;
			},
		}),
	});
});

export const layer = (options: Options = {}): Layer.Layer<Client> =>
	Layer.effect(Client, make(options));

const decodeActionErrorEnvelope = Schema.decodeUnknownEffect(
	ActionErrorEnvelope.ActionErrorEnvelope,
);

/** @internal */
export const makeRivetkitActionFailureClassifier = <
	ActionErrorSchema extends Schema.Codec<unknown, unknown, unknown, unknown>,
>(
	actionErrorSchema: ActionErrorSchema,
): ((
	cause: unknown,
) => Effect.Effect<
	ActionErrorSchema["Type"] | RivetError.RivetError,
	never,
	ActionErrorSchema["DecodingServices"]
>) => {
	const decodeActionError = Schema.decodeUnknownEffect(
		Schema.toCodecJson(actionErrorSchema),
	);

	return Effect.fnUntraced(function* (
		cause: unknown,
	): Effect.fn.Return<
		ActionErrorSchema["Type"] | RivetError.RivetError,
		never,
		ActionErrorSchema["DecodingServices"]
	> {
		// In the case where the `cause` is not a `RivetError`. In principle, this shouldn't happen.
		if (!RivetkitErrors.isRivetErrorLike(cause)) {
			return RivetError.fromUnknown(cause);
		}

		const rivetkitRivetError = RivetkitErrors.toRivetError(cause);

		const actionErrorEnvelope = yield* Effect.result(
			decodeActionErrorEnvelope(rivetkitRivetError.metadata),
		);

		// If the error's `metadata` is not a valid action error envelope, then
		// it means it's not a user-declared action error.
		if (Result.isFailure(actionErrorEnvelope)) {
			return RivetError.fromRivetkitRivetError(rivetkitRivetError);
		}

		const actionErrorResult = yield* Effect.result(
			decodeActionError(actionErrorEnvelope.success.error),
		);

		// The envelope was valid, but the inner payload doesn't match the
		// declared schema — surface as `ActionErrorDecodeFailed`
		if (Result.isFailure(actionErrorResult)) {
			return new RivetError.RivetError({
				reason: new RivetError.ActionErrorDecodeFailed({
					cause: actionErrorResult.failure,
					rivetError: rivetkitRivetError,
				}),
			});
		}

		// Successfully decoded user-declared action error
		return actionErrorResult.success;
	});
};
