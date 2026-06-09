import common from "@rivet-dev/agent-os-common";
import pi from "@rivet-dev/agent-os-pi";
import { agentOs } from "rivetkit/agent-os";

export const agentOsTestActor = agentOs({
	options: { software: [common, pi] },
});
