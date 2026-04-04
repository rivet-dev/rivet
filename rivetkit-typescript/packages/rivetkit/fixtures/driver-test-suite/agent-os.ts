import common from "@rivet-dev/agent-os-common";
import type { AgentOsOptions } from "@rivet-dev/agent-os-core";
import { agentOs } from "rivetkit/agent-os";

export const agentOsTestActor = agentOs({ options: { software: [common] } });

// Actor configured with acpTimeoutMs to verify the option passes through to
// AgentOs.create() without errors.
// TODO(#4552): Remove the type assertion once @rivet-dev/agent-os-core is
// published with acpTimeoutMs in AgentOsOptions.
export const agentOsTimeoutTestActor = agentOs({
	options: {
		software: [common],
		acpTimeoutMs: 300_000,
	} as AgentOsOptions,
});
