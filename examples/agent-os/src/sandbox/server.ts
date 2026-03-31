// Sandbox extension: mount a remote Docker sandbox into the VM.
//
// Requires Docker running locally. The sandbox-agent package manages the
// container lifecycle. The sandbox filesystem is mounted at /sandbox and
// the toolkit exposes process management as CLI commands.

import { agentOs } from "rivetkit/agent-os";
import { setup } from "rivetkit";
import common from "@rivet-dev/agent-os-common";
import { SandboxAgent } from "sandbox-agent";
import { docker } from "sandbox-agent/docker";
import {
	createSandboxFs,
	createSandboxToolkit,
} from "@rivet-dev/agent-os-sandbox";

// Start a Docker-backed sandbox.
const sandbox = await SandboxAgent.start({
	sandbox: docker(),
});

const vm = agentOs({
	options: {
		software: [common],
		mounts: [
			{
				path: "/sandbox",
				driver: createSandboxFs({ client: sandbox }),
			},
		],
		toolKits: [createSandboxToolkit({ client: sandbox })],
	},
});

export const registry = setup({ use: { vm } });
registry.start();
