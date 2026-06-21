import common from "@rivet-dev/agent-os-common";
import opencode from "@rivet-dev/agent-os-opencode";
import { setup } from "rivetkit";
import { agentOs } from "rivetkit/agent-os";

const llmockPort = Number(process.env.E2E_LLMOCK_PORT ?? "41235");
const opencodePackageDir = (opencode as { packageDir: string }).packageDir;

const vm = agentOs({
	options: {
		software: [common, opencode],
		mounts: [
			{
				path: "/root/node_modules/@rivet-dev/agent-os-opencode",
				plugin: {
					id: "host_dir",
					config: { hostPath: opencodePackageDir, readOnly: true },
				},
				readOnly: true,
			},
		],
		loopbackExemptPorts: [llmockPort],
	},
});

export const registry = setup({ use: { vm } });
registry.start();
