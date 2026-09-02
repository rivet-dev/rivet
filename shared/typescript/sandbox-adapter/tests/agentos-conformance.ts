import type { AgentOsActions } from "@rivet-dev/agentos";
import type { AgentOSActorHandle } from "../src/agentos.js";

type RemoteActions<T> = T extends (...args: infer TArgs) => infer TResult
	? TArgs extends [unknown, ...infer TRemoteArgs]
		? (...args: TRemoteArgs) => Promise<Awaited<TResult>>
		: never
	: T extends object
		? { [K in keyof T]: RemoteActions<T[K]> }
		: never;

type CanonicalAgentOSHandle = Pick<
	RemoteActions<AgentOsActions>,
	"process" | "filesystem"
>;

declare const canonicalHandle: CanonicalAgentOSHandle;

// Fails type checking when the adapter drifts from agentOS's canonical actions.
const adapterHandle: Pick<AgentOSActorHandle, "process" | "filesystem"> =
	canonicalHandle;

void adapterHandle;
