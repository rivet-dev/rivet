import { getLogger } from "@rivet-gg/actor-common/log";

/** Logger for this library. */
export const RUNTIME_LOGGER_NAME = "actor-runtime";

/** Logger used for logs from the actor instance itself. */
export const ACTOR_LOGGER_NAME = "actor";

/** Logger used for logs from the actor inspector. */
export const INSPECT_LOGGER_NAME = "actor-inspect";

export function logger() {
	return getLogger(RUNTIME_LOGGER_NAME);
}

export function instanceLogger() {
	return getLogger(ACTOR_LOGGER_NAME);
}

/**
 * Get the logger for the actor inspector.
 * @internal
 */
export function inspectLogger() {
	return getLogger(INSPECT_LOGGER_NAME);
}
