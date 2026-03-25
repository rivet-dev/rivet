import type { Context as HonoContext } from "hono";
import * as errors from "@/actor/errors";
import { deconstructError, stringifyError } from "@/common/utils";
import type { ManagerDriver } from "@/driver-helpers/mod";
import type { ActorQuery } from "@/manager/protocol/query";
import { ActorSchedulingError } from "./errors";
import { logger } from "./log";

/**
 * Query the manager driver to get or create a actor based on the provided query
 */
export async function queryActor(
	c: HonoContext | undefined,
	query: ActorQuery,
	managerDriver: ManagerDriver,
): Promise<{ actorId: string }> {
	logger().debug({ msg: "querying actor", query: JSON.stringify(query) });
	let actorOutput: { actorId: string };
	if ("getForId" in query) {
		const output = await managerDriver.getForId({
			c,
			name: query.getForId.name,
			actorId: query.getForId.actorId,
		});
		if (!output) throw new errors.ActorNotFound(query.getForId.actorId);
		actorOutput = output;
	} else if ("getForKey" in query) {
		const existingActor = await managerDriver.getWithKey({
			c,
			name: query.getForKey.name,
			key: query.getForKey.key,
		});
		if (!existingActor) {
			throw new errors.ActorNotFound(
				`${query.getForKey.name}:${JSON.stringify(query.getForKey.key)}`,
			);
		}
		actorOutput = existingActor;
	} else if ("getOrCreateForKey" in query) {
		const getOrCreateOutput = await managerDriver.getOrCreateWithKey({
			c,
			name: query.getOrCreateForKey.name,
			key: query.getOrCreateForKey.key,
			input: query.getOrCreateForKey.input,
			region: query.getOrCreateForKey.region,
		});
		actorOutput = {
			actorId: getOrCreateOutput.actorId,
		};
	} else if ("create" in query) {
		const createOutput = await managerDriver.createActor({
			c,
			name: query.create.name,
			key: query.create.key,
			input: query.create.input,
			region: query.create.region,
		});
		actorOutput = {
			actorId: createOutput.actorId,
		};
	} else {
		throw new errors.InvalidRequest("Invalid query format");
	}

	logger().debug({ msg: "actor query result", actorId: actorOutput.actorId });
	return { actorId: actorOutput.actorId };
}

/**
 * Extract the actor name from a query.
 */
export function getActorNameFromQuery(query: ActorQuery): string {
	if ("getForId" in query) return query.getForId.name;
	if ("getForKey" in query) return query.getForKey.name;
	if ("getOrCreateForKey" in query) return query.getOrCreateForKey.name;
	if ("create" in query) return query.create.name;
	throw new errors.InvalidRequest("Invalid query format");
}

export interface ActorResolutionState {
	actorQuery: ActorQuery;
	resolvedActorId?: string;
	pendingResolve?: Promise<string>;
}

export function createActorResolutionState(
	actorQuery: ActorQuery,
): ActorResolutionState {
	return { actorQuery };
}

function isLazyResolvableActorQuery(
	actorQuery: ActorQuery,
): actorQuery is
	| Extract<ActorQuery, { getForKey: unknown }>
	| Extract<ActorQuery, { getOrCreateForKey: unknown }> {
	return "getForKey" in actorQuery || "getOrCreateForKey" in actorQuery;
}

export async function resolveActorId(
	state: ActorResolutionState,
	driver: ManagerDriver,
): Promise<string> {
	if ("getForId" in state.actorQuery) {
		return state.actorQuery.getForId.actorId;
	}

	if (!isLazyResolvableActorQuery(state.actorQuery)) {
		const { actorId } = await queryActor(
			undefined,
			state.actorQuery,
			driver,
		);
		return actorId;
	}

	if (state.resolvedActorId !== undefined) {
		return state.resolvedActorId;
	}

	if (state.pendingResolve) {
		return await state.pendingResolve;
	}

	const resolvePromise = queryActor(undefined, state.actorQuery, driver)
		.then(({ actorId }) => {
			state.resolvedActorId = actorId;
			state.pendingResolve = undefined;
			return actorId;
		})
		.catch((err) => {
			// Clear the pending promise on failure so the next caller starts
			// a fresh resolve instead of re-awaiting a rejected promise.
			if (state.pendingResolve === resolvePromise) {
				state.pendingResolve = undefined;
			}
			throw err;
		});
	state.pendingResolve = resolvePromise;

	return await resolvePromise;
}

export function setResolvedActorId(
	state: ActorResolutionState,
	actorId: string,
): void {
	if (!isLazyResolvableActorQuery(state.actorQuery)) {
		return;
	}

	state.resolvedActorId = actorId;
	state.pendingResolve = undefined;
}

export function shouldInvalidateResolvedActorId(
	group: string,
	code: string,
): boolean {
	return (
		group === "actor" &&
		(code === "not_found" || code.startsWith("destroyed_"))
	);
}

/**
 * Invalidates the cached resolved actor ID when an error proves the cached
 * resolution is stale.
 *
 * This only clears cached resolutions for `.get()` and `.getOrCreate()`.
 * `getForId()` handles and connections always keep their explicit actor ID.
 *
 * Returns `true` when the error invalidated the cached resolution.
 */
export function invalidateResolvedActorIdFromError(
	state: ActorResolutionState,
	error: unknown,
): boolean {
	const { group, code } = deconstructError(error, logger(), {}, true);
	if (!shouldInvalidateResolvedActorId(group, code)) {
		return false;
	}

	invalidateResolvedActorId(state);
	return true;
}

export function invalidateResolvedActorId(state: ActorResolutionState): void {
	if (!isLazyResolvableActorQuery(state.actorQuery)) {
		return;
	}

	state.resolvedActorId = undefined;
	state.pendingResolve = undefined;
}

/**
 * Retries an operation once if the error indicates the cached actor resolution
 * is stale. On the first invalidatable error, clears the cached resolution and
 * re-runs. On the second failure (or a non-invalidatable error), throws.
 *
 * @param onInvalidate Optional callback invoked after the cached resolution is
 * cleared, allowing callers to perform additional cleanup (e.g. clearing
 * connection-specific state).
 */
export async function retryOnInvalidResolvedActor<T>(
	state: ActorResolutionState,
	run: () => Promise<T>,
	onInvalidate?: () => void,
): Promise<T> {
	let retried = false;

	while (true) {
		try {
			return await run();
		} catch (error) {
			if (retried || !invalidateResolvedActorIdFromError(state, error)) {
				throw error;
			}

			onInvalidate?.();
			retried = true;
		}
	}
}

/**
 * Fetch actor details and check for scheduling errors.
 */
export async function checkForSchedulingError(
	group: string,
	code: string,
	actorId: string,
	query: ActorQuery,
	driver: ManagerDriver,
): Promise<ActorSchedulingError | null> {
	const name = getActorNameFromQuery(query);

	try {
		const actor = await driver.getForId({ name, actorId });

		if (actor?.error) {
			logger().info({
				msg: "found actor scheduling error",
				actorId,
				error: actor.error,
			});
			return new ActorSchedulingError(group, code, actorId, actor.error);
		}
	} catch (err) {
		logger().warn({
			msg: "failed to fetch actor details for scheduling error check",
			actorId,
			error: stringifyError(err),
		});
	}

	return null;
}
