import { agentOs } from "rivetkit/agent-os";
import { setup } from "rivetkit";
import common from "@rivet-dev/agent-os-common";

const vm = agentOs({ options: { software: [common] } });

export const registry = setup({ use: { vm } });
registry.start();
