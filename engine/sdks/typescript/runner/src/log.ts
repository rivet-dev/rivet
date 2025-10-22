import { Logger } from "pino";

let LOGGER: Logger | undefined = undefined;

export function setLogger(logger: Logger) {
	LOGGER = logger;
}

export function logger(): Logger | undefined {
	return LOGGER;
}
