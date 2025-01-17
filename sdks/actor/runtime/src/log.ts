import { getLogger } from "@rivet-gg/actor-common/log";

/** Logger for this library. */
export const RUNTIME_LOGGER_NAME = "actor-runtime";

/** Logger used for logs from the actor instance itself. */
export const ACTOR_LOGGER_NAME = "actor";

export function logger() {
	return getLogger(RUNTIME_LOGGER_NAME);
}

export function instanceLogger() {
	return getLogger(ACTOR_LOGGER_NAME);
}
