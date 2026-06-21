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
const llmockPort = Number(process.env.E2E_LLMOCK_PORT ?? "41235");

// The client exercises session resume by forcing the actor to sleep (engine admin
// POST /actors/{id}/sleep) between two prompts in the same session. Sleep tears
// down the VM and clears the actor's ephemeral `live_sessions` map, so the second
// prompt lazily reconstructs the session transcript from `agent_os_session_events`
// and resumes -- proving resume survives a real sleep/wake.
const vm = agentOs({
	options: {
		software: [common, pi],
		mounts: [nodeModulesMount(agentModules)],
		loopbackExemptPorts: [llmockPort],
	},
});

export const registry = setup({ use: { vm } });
registry.start();
