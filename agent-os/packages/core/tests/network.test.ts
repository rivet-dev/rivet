import { afterEach, beforeEach, describe, expect, test } from "vitest";
import { AgentOs } from "../src/index.js";

const SERVER_SCRIPT = `
const http = require("http");
const server = http.createServer((req, res) => {
  res.writeHead(200, { "Content-Type": "application/json" });
  res.end(JSON.stringify({ status: "ok", method: req.method, url: req.url }));
});
server.listen(0, "0.0.0.0", () => {
  const port = server.address().port;
  console.log("LISTENING:" + port);
});
`;

describe("networking", () => {
	let vm: AgentOs;

	beforeEach(async () => {
		vm = await AgentOs.create();
	});

	afterEach(async () => {
		await vm.dispose();
	});

	test("fetch JSON from HTTP server running inside VM", async () => {
		await vm.writeFile("/tmp/server.js", SERVER_SCRIPT);

		let resolvePort: (port: number) => void;
		const portPromise = new Promise<number>((resolve) => {
			resolvePort = resolve;
		});

		const { pid } = vm.spawn("node", ["/tmp/server.js"], {
			onStdout: (data: Uint8Array) => {
				const text = new TextDecoder().decode(data);
				const match = text.match(/LISTENING:(\d+)/);
				if (match) {
					resolvePort(Number(match[1]));
				}
			},
		});

		const port = await portPromise;

		const response = await vm.fetch(
			port,
			new Request("http://localhost/test"),
		);
		expect(response.ok).toBe(true);

		const json = await response.json();
		expect(json).toEqual({
			status: "ok",
			method: "GET",
			url: "/test",
		});

		// Kill server process; dispose() in afterEach handles full cleanup
		vm.killProcess(pid);
	});
});
