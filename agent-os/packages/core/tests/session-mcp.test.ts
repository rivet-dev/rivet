import { afterAll, beforeAll, describe, expect, test } from "vitest";
import { AcpClient } from "../src/acp-client.js";
import type { McpServerConfig } from "../src/agent-os.js";
import { AgentOs } from "../src/agent-os.js";
import { createStdoutLineIterable } from "../src/stdout-lines.js";

/**
 * Mock ACP adapter that echoes back the full mcpServers array
 * from session/new params in the response, enabling exact
 * serialization assertions. Supports multiple session/new calls
 * via session counter.
 */
const MCP_ECHO_MOCK = `
let buffer = '';
let sessionCounter = 0;

process.stdin.resume();
process.stdin.on('data', (chunk) => {
  const str = chunk instanceof Uint8Array ? new TextDecoder().decode(chunk) : String(chunk);
  buffer += str;

  while (true) {
    const idx = buffer.indexOf('\\n');
    if (idx === -1) break;
    const line = buffer.substring(0, idx);
    buffer = buffer.substring(idx + 1);
    if (!line.trim()) continue;

    try {
      const msg = JSON.parse(line);
      if (msg.id === undefined) continue;

      let result;
      switch (msg.method) {
        case 'initialize':
          result = { protocolVersion: 1 };
          break;

        case 'session/new': {
          sessionCounter++;
          const params = msg.params || {};
          result = {
            sessionId: 'mcp-session-' + sessionCounter,
            receivedMcpServers: params.mcpServers,
            hasMcpServersKey: 'mcpServers' in params,
          };
          break;
        }

        default:
          process.stdout.write(JSON.stringify({
            jsonrpc: '2.0', id: msg.id,
            error: { code: -32601, message: 'Method not found' },
          }) + '\\n');
          continue;
      }

      process.stdout.write(JSON.stringify({
        jsonrpc: '2.0', id: msg.id, result,
      }) + '\\n');
    } catch (e) {}
  }
});
`;

async function newSession(client: AcpClient, params: Record<string, unknown>) {
	const resp = await client.request("session/new", params);
	if (resp.error)
		throw new Error(`session/new failed: ${resp.error.message}`);
	return resp.result as {
		sessionId: string;
		receivedMcpServers: unknown;
		hasMcpServersKey: boolean;
	};
}

describe("MCP server config passthrough", () => {
	let vm: AgentOs;
	let client: AcpClient;

	beforeAll(async () => {
		vm = await AgentOs.create();
		const { iterable, onStdout } = createStdoutLineIterable();
		await vm.writeFile("/tmp/mcp-echo.mjs", MCP_ECHO_MOCK);
		const proc = vm.kernel.spawn("node", ["/tmp/mcp-echo.mjs"], {
			streamStdin: true,
			onStdout,
			env: { HOME: "/home/user" },
		});
		client = new AcpClient(proc, iterable);
		const initResp = await client.request("initialize", {
			protocolVersion: 1,
			clientCapabilities: {},
		});
		if (initResp.error)
			throw new Error(`initialize failed: ${initResp.error.message}`);
	}, 30_000);

	afterAll(async () => {
		await vm.dispose();
	});

	test("createSession with mcpServers includes mcpServers in session/new params", async () => {
		const mcpServers: McpServerConfig[] = [
			{
				type: "local",
				command: "node",
				args: ["/path/to/server.js"],
			},
		];

		const result = await newSession(client, {
			cwd: "/home/user",
			mcpServers,
		});

		expect(result.hasMcpServersKey).toBe(true);
		expect(result.receivedMcpServers).toEqual(mcpServers);
	}, 15_000);

	test("local MCP server config with command, args, env is serialized correctly", async () => {
		const mcpServers: McpServerConfig[] = [
			{
				type: "local",
				command: "node",
				args: ["/path/to/mcp-server.js", "--port", "3000"],
				env: { LOG_LEVEL: "debug", NODE_ENV: "test" },
			},
		];

		const result = await newSession(client, {
			cwd: "/home/user",
			mcpServers,
		});

		expect(result.hasMcpServersKey).toBe(true);
		expect(result.receivedMcpServers).toEqual(mcpServers);
	}, 15_000);

	test("remote MCP server config with url and headers is serialized correctly", async () => {
		const mcpServers: McpServerConfig[] = [
			{
				type: "remote",
				url: "https://mcp.example.com/v1",
				headers: {
					Authorization: "Bearer test-token",
					"X-Custom-Header": "value",
				},
			},
		];

		const result = await newSession(client, {
			cwd: "/home/user",
			mcpServers,
		});

		expect(result.hasMcpServersKey).toBe(true);
		expect(result.receivedMcpServers).toEqual(mcpServers);
	}, 15_000);

	test("empty mcpServers array is passed through, not omitted", async () => {
		const result = await newSession(client, {
			cwd: "/home/user",
			mcpServers: [],
		});

		expect(result.hasMcpServersKey).toBe(true);
		expect(result.receivedMcpServers).toEqual([]);
	}, 15_000);

	test("session without mcpServers option does not include mcpServers in params", async () => {
		const result = await newSession(client, {
			cwd: "/home/user",
		});

		expect(result.hasMcpServersKey).toBe(false);
		expect(result.receivedMcpServers).toBeUndefined();
	}, 15_000);

	test("multiple MCP servers of mixed types are serialized correctly", async () => {
		const mcpServers: McpServerConfig[] = [
			{
				type: "local",
				command: "npx",
				args: ["-y", "@modelcontextprotocol/server-filesystem"],
			},
			{
				type: "remote",
				url: "https://mcp.example.com/tools",
				headers: { Authorization: "Bearer abc123" },
			},
			{
				type: "local",
				command: "python",
				args: ["-m", "mcp_server"],
				env: { PYTHONPATH: "/opt/lib" },
			},
		];

		const result = await newSession(client, {
			cwd: "/home/user",
			mcpServers,
		});

		expect(result.hasMcpServersKey).toBe(true);
		expect(result.receivedMcpServers).toEqual(mcpServers);
	}, 15_000);

	test("local config with minimal fields (no args, no env)", async () => {
		const mcpServers: McpServerConfig[] = [
			{
				type: "local",
				command: "mcp-server",
			},
		];

		const result = await newSession(client, {
			cwd: "/home/user",
			mcpServers,
		});

		expect(result.hasMcpServersKey).toBe(true);
		expect(result.receivedMcpServers).toEqual(mcpServers);
	}, 15_000);

	test("remote config with minimal fields (no headers)", async () => {
		const mcpServers: McpServerConfig[] = [
			{
				type: "remote",
				url: "https://mcp.example.com",
			},
		];

		const result = await newSession(client, {
			cwd: "/home/user",
			mcpServers,
		});

		expect(result.hasMcpServersKey).toBe(true);
		expect(result.receivedMcpServers).toEqual(mcpServers);
	}, 15_000);
});
