// Sandbox mounting: each actor instance gets its own Docker sandbox.
//
// Requires Docker running locally. The sandbox-agent package manages the
// container lifecycle. The sandbox filesystem is mounted at /sandbox and
// the toolkit exposes process management as CLI commands.
//
// Using `createOptions` ensures every actor instance spawned via
// `client.vm.getOrCreate(...)` provisions a dedicated sandbox so
// multiple agents never share the same container.

import common from "@rivet-dev/agent-os-common";
import {
	createSandboxFs,
	createSandboxToolkit,
} from "@rivet-dev/agent-os-sandbox";
import { setup } from "rivetkit";
import { agentOs } from "rivetkit/agent-os";
import { SandboxAgent } from "sandbox-agent";
import { docker } from "sandbox-agent/docker";

const vm = agentOs({
	createOptions: async (c) => {
		c.log.info({ msg: "provisioning sandbox for actor instance" });

		// Start a dedicated Docker-backed sandbox for this actor instance.
		const sandbox = await SandboxAgent.start({
			sandbox: docker(),
		});

		return {
			software: [common],
			mounts: [
				{
					path: "/sandbox",
					driver: createSandboxFs({ client: sandbox }),
				},
			],
			toolKits: [createSandboxToolkit({ client: sandbox })],
		};
	},
});

export const registry = setup({ use: { vm } });
registry.start();
