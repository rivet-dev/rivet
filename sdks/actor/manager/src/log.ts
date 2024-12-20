import { getLogger } from "../../common/src/log.ts";

export const LOGGER_NAME = "actor-manager";

export function logger() {
	return getLogger(LOGGER_NAME);
}
