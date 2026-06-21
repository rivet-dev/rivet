import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import common from "@rivet-dev/agent-os-common";
import { agentOs, nodeModulesMount } from "rivetkit/agent-os";

const here = dirname(fileURLToPath(import.meta.url));
const mockNodeModules = join(here, "..", "agent-os-mock-modules");
const mockOpencodePackage = join(
	mockNodeModules,
	"@rivet-dev",
	"agent-os-opencode",
);
const mockOpencode = {
	packageDir: mockOpencodePackage,
	agent: {
		id: "opencode",
		acpAdapter: "@rivet-dev/agent-os-opencode",
		agentPackage: "@rivet-dev/agent-os-opencode",
	},
};

export const agentOsTestActor = agentOs({
	options: {
		software: [common, mockOpencode],
		mounts: [nodeModulesMount(mockNodeModules)],
	},
});
