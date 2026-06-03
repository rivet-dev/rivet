import common from "@rivet-dev/agent-os-common";
import git from "@rivet-dev/agent-os-git";
import { setup } from "rivetkit";
import { agentOs } from "rivetkit/agent-os";

const vm = agentOs({ options: { software: [common, git] } });

export const registry = setup({ use: { vm } });
registry.start();
