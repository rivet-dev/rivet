import { getLogger } from "../../common/src/log.ts";

/** Logger for this library. */
export const LOGGER_NAME = "actors";

/** Logger used for logs from the actor instance itself. */
export const INSTANCE_LOGGER_NAME = "actors-instance";

export function logger() {
	return getLogger(LOGGER_NAME);
}

export function instanceLogger() {
	return getLogger(INSTANCE_LOGGER_NAME);
}
