import { afterAll, beforeAll, describe } from "vitest";
import { defineFsDriverTests } from "@rivet-dev/agent-os/test/file-system";
import type { SandboxAgentContainerHandle } from "@rivet-dev/agent-os/test/docker";
import { startSandboxAgentContainer } from "@rivet-dev/agent-os/test/docker";
import { createSandboxBackend } from "../src/index.js";

let sandbox: SandboxAgentContainerHandle;

const skipReason = process.env.SKIP_SANDBOX_TESTS
	? "SKIP_SANDBOX_TESTS is set"
	: undefined;

beforeAll(async () => {
	if (skipReason) return;
	sandbox = await startSandboxAgentContainer({ healthTimeout: 120_000 });
}, 150_000);

afterAll(async () => {
	if (sandbox) await sandbox.stop();
});

describe.skipIf(skipReason)("sandbox-backend", () => {
	defineFsDriverTests({
		name: "SandboxBackend",
		createFs: () => {
			return createSandboxBackend({ client: sandbox.client });
		},
		capabilities: {
			symlinks: false,
			hardLinks: false,
			permissions: false,
			utimes: false,
			truncate: true,
			pread: true,
			mkdir: true,
			removeDir: true,
		},
	});
});
