// Sandbox mounting: each actor instance gets its own Docker sandbox.
//
// Requires Docker running locally. The sandbox-agent package manages the
// container lifecycle. The sandbox filesystem is mounted at /sandbox and
// the toolkit exposes process management as CLI commands.
//
// Using `createOptions` ensures every actor instance spawned via
// `client.vm.getOrCreate(...)` provisions a dedicated sandbox so
// multiple agents never share the same container.
//
// The sandbox ID is persisted in `c.state.sandboxId` so that after a
// sleep/wake cycle the actor reconnects to the same container instead of
// creating a new one.

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
		c.log.info({
			msg: "booting sandbox",
			existingSandboxId: c.state.sandboxId,
		});

		// Reconnect to an existing sandbox after sleep, or provision a new one.
		const sandbox = await SandboxAgent.start({
			sandbox: docker(),
			sandboxId: c.state.sandboxId ?? undefined,
		});

		// Persist the sandbox ID so future wakes reuse the same container.
		c.state.sandboxId = sandbox.sandboxId;

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
	destroyOptions: async (c) => {
		// Destroy the sandbox container when the actor is destroyed.
		if (c.state.sandboxId) {
			const provider = docker();
			await provider.destroy(c.state.sandboxId);
		}
	},
});

export const registry = setup({ use: { vm } });
registry.start();
