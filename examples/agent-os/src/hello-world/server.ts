import common from "@agent-os-pkgs/common";
import { setup } from "rivetkit";
import { agentOs } from "rivetkit/agent-os";

const vm = agentOs({ options: { software: [common] } });

export const registry = setup({ use: { vm } });
registry.start();
