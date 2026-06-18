import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import common from "@rivet-dev/agent-os-common";
import pi from "@rivet-dev/agent-os-pi";
import { setup } from "rivetkit";
import { agentOs, nodeModulesMount } from "rivetkit/agent-os";

// The Pi agent closure is pre-installed (flat npm tree) into `.agent-modules/`
// by `scripts/prepare-agent-modules.mjs`. Mount its `node_modules` at
// `/root/node_modules` so the VM module resolver can read the agent SDK + its
// transitive deps through the kernel VFS.
const here = dirname(fileURLToPath(import.meta.url));
const agentModules = join(here, "..", ".agent-modules", "node_modules");

const vm = agentOs({
	options: {
		software: [common, pi],
		mounts: [nodeModulesMount(agentModules)],
	},
});

export const registry = setup({ use: { vm } });
registry.start();
