import * as errors from "./errors.ts";

export function assertUnreachable(x: never): never {
	throw new errors.Unreachable(x);
}
