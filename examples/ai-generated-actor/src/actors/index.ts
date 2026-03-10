import { setup } from "rivetkit";
import { codeAgent } from "./code-agent.ts";
import { dynamicRunner } from "./dynamic-runner.ts";

export type {
	ChatMessage,
	CodeAgentState,
	CodeUpdateEvent,
	ResponseEvent,
} from "./code-agent.ts";

export { DEFAULT_ACTOR_CODE } from "./code-agent.ts";

// Register actors for use: https://rivet.dev/docs/setup
export const registry = setup({
	use: { codeAgent, dynamicRunner },
});
