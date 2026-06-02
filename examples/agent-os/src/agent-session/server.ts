import common from "@rivet-dev/agent-os-common";
import pi from "@rivet-dev/agent-os-pi";
import { setup } from "rivetkit";
import { agentOs } from "rivetkit/agent-os";

const vm = agentOs({ options: { software: [common, pi] } });

export const registry = setup({ use: { vm } });
registry.start();
