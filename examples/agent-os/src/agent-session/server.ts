import { agentOs } from "rivetkit/agent-os";
import { setup } from "rivetkit";
import common from "@rivet-dev/agent-os-common";
import pi from "@rivet-dev/agent-os-pi";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const vm = agentOs({ options: { software: [common, pi] } }) as any;

export const registry = setup({ use: { vm } });
registry.start();
