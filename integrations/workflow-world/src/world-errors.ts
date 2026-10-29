import {
	EntityConflictError,
	HookNotFoundError,
	RunExpiredError,
	TooEarlyError,
	WorkflowError,
	WorkflowRunNotFoundError,
	WorkflowWorldError,
} from "@workflow/errors";
import { UserError } from "rivetkit";
import { ActorError } from "rivetkit/client";

// RivetKit returns only UserError details to callers and turns every other
// actor error into `internal_error`. The run actor therefore sends Workflow SDK
// errors as UserErrors coded by class name, and the World client rebuilds them
// so the runtime's `EntityConflictError.is(...)` style checks still match.

type WorldErrorFields = { runId?: string; token?: string; retryAfter?: number };

export async function exposeWorldErrors<T>(body: () => Promise<T>): Promise<T> {
	try {
		return await body();
	} catch (error) {
		if (!(error instanceof WorkflowError)) throw error;
		const { runId, token, retryAfter } = error as WorldErrorFields;
		throw new UserError(error.message, {
			code: error.name,
			metadata: { runId, token, retryAfter },
		});
	}
}

export function restoreWorldError(error: unknown): unknown {
	if (!(error instanceof ActorError) || error.group !== "user") return error;
	const { runId, token, retryAfter } = (error.metadata ?? {}) as WorldErrorFields;
	switch (error.code) {
		case "EntityConflictError":
			return new EntityConflictError(error.message);
		case "RunExpiredError":
			return new RunExpiredError(error.message, runId);
		case "TooEarlyError":
			return new TooEarlyError(error.message, { retryAfter });
		case "HookNotFoundError":
			return new HookNotFoundError(token ?? "");
		case "WorkflowRunNotFoundError":
			return new WorkflowRunNotFoundError(runId ?? "");
		case "WorkflowWorldError":
			return new WorkflowWorldError(error.message);
		default:
			return error;
	}
}

/** Makes every action call on `handle` reject with the rebuilt World error. */
export function withWorldErrors<T extends object>(handle: T): T {
	return new Proxy(handle, {
		get(target, prop, receiver) {
			const value = Reflect.get(target, prop, receiver);
			if (typeof value !== "function") return value;
			return (...args: unknown[]) => {
				// RivetKit action methods are proxies, so `value.apply` would resolve
				// to an action named `<name>.apply`. Reflect.apply calls it directly.
				const result = Reflect.apply(value, target, args);
				return result instanceof Promise
					? result.catch((error: unknown) => {
							throw restoreWorldError(error);
						})
					: result;
			};
		},
	});
}
