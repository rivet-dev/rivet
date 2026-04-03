import common from "@rivet-dev/agent-os-common";
import { agentOs } from "rivetkit/agent-os";

export const agentOsTestActor = agentOs({ options: { software: [common] } });

// Same actor using the per-instance createOptions factory path. The factory
// returns identical options to the static fixture above so the driver tests
// can verify both paths produce a working VM.
export const agentOsCreateOptionsTestActor = agentOs({
	createOptions: async () => ({ software: [common] }),
});
