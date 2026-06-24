import common from "@agent-os-pkgs/common";
import git from "@rivet-dev/agent-os-git";
import { setup } from "rivetkit";
import { agentOs } from "rivetkit/agent-os";

const vm = agentOs({ options: { software: [common, git] } });

export const registry = setup({ use: { vm } });
registry.start();
