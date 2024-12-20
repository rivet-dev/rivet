import { getLogger } from "../common/log.ts";

export const LOGGER_NAME = "actor-client";

export function logger() {
	return getLogger(LOGGER_NAME);
}
