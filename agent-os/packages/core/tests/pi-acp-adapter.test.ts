import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import type { LLMock } from "@copilotkit/llmock";
import type { ManagedProcess } from "@secure-exec/core";
import {
	afterAll,
	afterEach,
	beforeAll,
	beforeEach,
	describe,
	expect,
	test,
} from "vitest";
import { AcpClient } from "../src/acp-client.js";
import { AgentOs } from "../src/agent-os.js";
import { createStdoutLineIterable } from "../src/stdout-lines.js";
import {
	DEFAULT_TEXT_FIXTURE,
	startLlmock,
	stopLlmock,
} from "./helpers/llmock-helper.js";

/**
 * Workspace root has shamefully-hoisted node_modules with pi-acp available.
 */
const MODULE_ACCESS_CWD = resolve(import.meta.dirname, "..");

/**
 * Resolve pi-acp bin path from host node_modules.
 * kernel.readFile() doesn't see the ModuleAccessFileSystem overlay,
 * so we read the host package.json directly and construct the VFS path.
 */
function resolvePiAcpBinPath(): string {
	const hostPkgJson = join(
		MODULE_ACCESS_CWD,
		"node_modules/pi-acp/package.json",
	);
	const pkg = JSON.parse(readFileSync(hostPkgJson, "utf-8"));

	let binEntry: string;
	if (typeof pkg.bin === "string") {
		binEntry = pkg.bin;
	} else if (typeof pkg.bin === "object" && pkg.bin !== null) {
		binEntry =
			(pkg.bin as Record<string, string>)["pi-acp"] ??
			Object.values(pkg.bin)[0];
	} else {
		throw new Error("No bin entry in pi-acp package.json");
	}

	return `/root/node_modules/pi-acp/${binEntry}`;
}

describe("pi-acp adapter manual spawn", () => {
	let vm: AgentOs;
	let mock: LLMock;
	let mockUrl: string;
	let mockPort: number;
	let client: AcpClient;

	beforeAll(async () => {
		const result = await startLlmock([DEFAULT_TEXT_FIXTURE]);
		mock = result.mock;
		mockUrl = result.url;
		mockPort = Number(new URL(result.url).port);
	});

	afterAll(async () => {
		await stopLlmock(mock);
	});

	beforeEach(async () => {
		vm = await AgentOs.create({
			loopbackExemptPorts: [mockPort],
			moduleAccessCwd: MODULE_ACCESS_CWD,
		});
	});

	afterEach(async () => {
		if (client) {
			client.close();
		}
		await vm.dispose();
	});

	/**
	 * Spawn pi-acp from the mounted node_modules overlay and wire up AcpClient.
	 */
	function spawnPiAcp(): {
		proc: ManagedProcess;
		client: AcpClient;
		stderr: () => string;
	} {
		const binPath = resolvePiAcpBinPath();
		const { iterable, onStdout } = createStdoutLineIterable();

		let stderrOutput = "";
		const spawned = vm.kernel.spawn("node", [binPath], {
			streamStdin: true,
			onStdout,
			onStderr: (data: Uint8Array) => {
				stderrOutput += new TextDecoder().decode(data);
			},
			env: {
				HOME: "/home/user",
				ANTHROPIC_API_KEY: "mock-key",
				ANTHROPIC_BASE_URL: mockUrl,
			},
		});

		const acpClient = new AcpClient(spawned, iterable);
		return { proc: spawned, client: acpClient, stderr: () => stderrOutput };
	}

	test("initialize returns protocolVersion and agentInfo", async () => {
		const spawned = spawnPiAcp();
		client = spawned.client;

		let response: Awaited<ReturnType<AcpClient["request"]>>;
		try {
			response = await client.request("initialize", {
				protocolVersion: 1,
				clientCapabilities: {},
			});
		} catch (err) {
			throw new Error(
				`Initialize failed. stderr: ${spawned.stderr()}\n${err}`,
			);
		}

		expect(
			response.error,
			`ACP error: ${JSON.stringify(response.error)}`,
		).toBeUndefined();
		expect(response.result).toBeDefined();

		const result = response.result as Record<string, unknown>;
		expect(result.protocolVersion).toBeDefined();
		expect(result.agentInfo).toBeDefined();

		const agentInfo = result.agentInfo as Record<string, unknown>;
		expect(agentInfo.name).toBeDefined();
	}, 60_000);

	test("session/new sends request and receives JSON-RPC response", async () => {
		const spawned = spawnPiAcp();
		client = spawned.client;

		// Must initialize first
		let initResponse: Awaited<ReturnType<AcpClient["request"]>>;
		try {
			initResponse = await client.request("initialize", {
				protocolVersion: 1,
				clientCapabilities: {},
			});
		} catch (err) {
			throw new Error(
				`Initialize failed. stderr: ${spawned.stderr()}\n${err}`,
			);
		}
		expect(initResponse.error).toBeUndefined();

		// Send session/new. pi-acp internally spawns the PI CLI as a child
		// process. The bare `pi` command can't be resolved inside the VM
		// (no shell PATH lookup), so pi-acp returns an error. We verify the
		// JSON-RPC protocol works correctly by checking the error response.
		let sessionResponse: Awaited<ReturnType<AcpClient["request"]>>;
		try {
			sessionResponse = await client.request("session/new", {
				cwd: "/home/user",
				mcpServers: [],
			});
		} catch (err) {
			throw new Error(
				`session/new failed. stderr: ${spawned.stderr()}\n${err}`,
			);
		}

		// Verify we got a well-formed JSON-RPC response (error expected since
		// pi-acp can't spawn the `pi` binary inside the VM without shell PATH)
		expect(sessionResponse.id).toBeDefined();
		expect(sessionResponse.jsonrpc).toBe("2.0");
		expect(sessionResponse.error).toBeDefined();
		expect(sessionResponse.error?.code).toBe(-32603);
		expect(sessionResponse.error?.data).toBeDefined();
	}, 60_000);
});
