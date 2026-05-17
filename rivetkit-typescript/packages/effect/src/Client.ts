import { Context, Effect, Exit, Layer, Record, Schema } from "effect";
import * as RivetkitClient from "rivetkit/client";
import type * as Action from "./Action";
import type * as Actor from "./Actor";
import * as RivetRivetError from "./internal/RivetRivetError";
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
					const decodeError = decodeRejectedActionCall(
						action.errorSchema,
					);

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
													cause: {
														group: "encoding",
														code: "invalid",
														message:
															"Could not encode action payload",
														metadata: cause,
													},
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
									decodeError(unknownError.cause),
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

const decodeRejectedActionCall = <E extends Schema.Top>(
	actionErrorSchema: E,
) => {
	const decodeRivetkitRivetError = Schema.decodeUnknownEffect(
		RivetRivetError.RivetkitRivetError,
	);
	const decodeActionErrorMetadata = Schema.decodeUnknownEffect(
		RivetRivetError.ActionErrorMetadata,
	);
	const decodeActionError = Schema.decodeUnknownEffect(
		Schema.toCodecJson(actionErrorSchema),
	);

	return Effect.fnUntraced(function* (cause: unknown) {
		const rivetkitRivetError = yield* decodeRivetkitRivetError(cause).pipe(
			Effect.mapError(() => RivetError.fromUnknown(cause)),
		);

		const actionErrorMetadata = yield* Effect.exit(
			decodeActionErrorMetadata(rivetkitRivetError.metadata),
		);

		if (Exit.isFailure(actionErrorMetadata)) {
			return yield* Effect.fail(
				RivetError.fromRivetkitRivetError(rivetkitRivetError),
			);
		}

		const actionError = yield* decodeActionError(
			actionErrorMetadata.value.error,
		).pipe(
			Effect.mapError(() =>
				RivetError.fromRivetkitRivetError(rivetkitRivetError),
			),
		);

		return yield* Effect.fail(actionError as E["Type"]);
	});
};
