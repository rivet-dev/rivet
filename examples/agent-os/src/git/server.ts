import { agentOs } from "rivetkit/agent-os";
import { setup } from "rivetkit";
import common from "@rivet-dev/agent-os-common";
import git from "@rivet-dev/agent-os-git";

const vm = agentOs({ options: { software: [common, git] } });

export const registry = setup({ use: { vm } });
registry.start();
