import { getLogger } from "@/common/log";

/** Prever to use ActorInstance.rlog child logger. This does not provide context in the log, should only be used as a last resort if you cannot pass the actor's child logger. */
export function loggerWithoutContext() {
	return getLogger("actor-runtime");
}
