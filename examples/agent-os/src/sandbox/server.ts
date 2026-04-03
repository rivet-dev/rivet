// Sandbox mounting: each actor instance gets its own Docker sandbox.
//
// Requires Docker running locally. The sandbox-agent package manages the
// container lifecycle. The sandbox filesystem is mounted at /sandbox and
// the toolkit exposes process management as CLI commands.
//
// Using `createOptions` with `c.getSandbox()` ensures every actor instance
// spawned via `client.vm.getOrCreate(...)` provisions a dedicated sandbox.
// The sandbox ID is persisted internally across sleep/wake so the actor
// reconnects to the same container. The sandbox is auto-destroyed when the
// actor is destroyed.

import common from "@rivet-dev/agent-os-common";
import { setup } from "rivetkit";
import { agentOs } from "rivetkit/agent-os";
import { docker } from "sandbox-agent/docker";

const vm = agentOs({
	createOptions: async (c) => {
		const { fs, toolkit } = await c.getSandbox({ provider: docker() });

		return {
			software: [common],
			mounts: [
				{
					path: "/sandbox",
					driver: fs,
				},
			],
			toolKits: [toolkit],
		};
	},
});

export const registry = setup({ use: { vm } });
registry.start();
