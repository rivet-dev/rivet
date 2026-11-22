/**
 * Calls from a cluster client to a cluster actor's actions.
 *
 * Portions adapted from Effect (MIT License,
 * Copyright (c) 2023 Effectful Technologies Inc). See `LICENSE-effect`.
 *
 * @internal
 */
import { PersistenceError } from "effect/cluster/ClusterError";
import * as Effect from "effect/Effect";
import * as Schedule from "effect/Schedule";
import { RivetError } from "rivetkit/errors";
import { entityStoppingCode } from "./entityActor.ts";
import { workflowStoppingCode } from "./workflowActor.ts";

/** A failed actor call, classified for the retry decision. */
export class ActorCallError extends Error {
	readonly retryable: boolean;
	readonly overloaded: boolean;

	constructor(cause: unknown) {
		super(cause instanceof Error ? cause.message : String(cause), {
			cause,
		});
		this.name = "ActorCallError";
		const classification = classify(cause);
		this.retryable = classification.retryable;
		this.overloaded = classification.overloaded;
	}
}

// Gateway and lifecycle codes after which the same request can safely be
// sent again. RivetKit's client already retries most lifecycle errors, so
// these are the ones that still surface.
const retryableCodes = new Set([
	`user.${entityStoppingCode}`,
	`user.${workflowStoppingCode}`,
	"actor.restarting",
	"actor.stopping",
	"guard.service_unavailable",
	"guard.actor_wake_retries_exceeded",
	"guard.actor_stopped_while_waiting",
	"guard.tunnel_request_aborted",
	"guard.tunnel_message_timeout",
	"guard.tunnel_response_closed",
	"guard.gateway_response_start_timeout",
]);

const overloadedCodes = new Set(["envoy.actor_overloaded", "actor.overloaded"]);

const classify = (
	cause: unknown,
): { readonly retryable: boolean; readonly overloaded: boolean } => {
	if (cause instanceof RivetError) {
		const code = `${cause.group}.${cause.code}`;
		return {
			retryable: retryableCodes.has(code),
			overloaded: overloadedCodes.has(code),
		};
	}
	// A request that never got a structured reply failed in transit.
	if (cause instanceof TypeError) {
		return { retryable: true, overloaded: false };
	}
	return { retryable: false, overloaded: false };
};

/** Whether a failed call may be sent again. Overload is never retried. */
export const isRetryable = (error: PersistenceError): boolean => {
	const cause = error.cause;
	return (
		cause instanceof ActorCallError && cause.retryable && !cause.overloaded
	);
};

/** Retries a persisted request four times with exponential backoff from 100 ms. */
export const boundedRetry: Schedule.Schedule<unknown> = Schedule.max([
	Schedule.exponential(100, 2),
	Schedule.recurs(4),
]);

/**
 * Calls one actor action. A fresh stub is resolved for every attempt. With
 * `retry`, only failures classified as retryable are sent again, so it must
 * only be used for calls that are idempotent on the actor.
 */
export const callActor = <Stub, A>(options: {
	readonly resolve: () => Stub;
	readonly call: (stub: Stub) => Promise<A>;
	readonly retry?: Schedule.Schedule<unknown, PersistenceError> | undefined;
}): Effect.Effect<A, PersistenceError> => {
	const attempt = Effect.tryPromise({
		try: () => options.call(options.resolve()),
		catch: (cause) =>
			new PersistenceError({ cause: new ActorCallError(cause) }),
	});
	return options.retry === undefined
		? attempt
		: Effect.retry(attempt, {
				schedule: options.retry,
				while: isRetryable,
			});
};
