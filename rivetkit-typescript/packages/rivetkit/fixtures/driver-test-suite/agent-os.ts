import { agentOs } from "rivetkit/agent-os";
import common from "@rivet-dev/agent-os-common";

export const agentOsTestActor = agentOs({ options: { software: [common] } });
