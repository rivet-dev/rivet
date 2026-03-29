import { agentOs } from "rivetkit/agent-os";
import { setup } from "rivetkit";
import common from "@rivet-dev/agent-os-common";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const vm = agentOs({ options: { software: [common] } }) as any;

export const registry = setup({ use: { vm } });
registry.start();
