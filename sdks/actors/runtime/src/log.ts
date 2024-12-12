import { getLogger } from "../../common/src/log.ts";

export const LOGGER_NAME = "actors";

export function logger() {
	return getLogger(LOGGER_NAME);
}
