import { afterEach, beforeEach, describe, expect, test } from "vitest";
import { AgentOs } from "../src/index.js";

describe("flat shell API", () => {
	let vm: AgentOs;

	beforeEach(async () => {
		vm = await AgentOs.create();
	});

	afterEach(async () => {
		await vm.dispose();
	});

	test("open shell, write command via writeShell, read output via onShellData", async () => {
		// Write a simple script that reads stdin and writes to stdout
		await vm.writeFile(
			"/tmp/shell-echo.mjs",
			`process.stdin.on("data", (chunk) => { process.stdout.write("GOT:" + chunk); });`,
		);

		const { shellId } = vm.openShell({ command: "node", args: ["/tmp/shell-echo.mjs"] });

		const chunks: string[] = [];
		vm.onShellData(shellId, (data) => {
			chunks.push(new TextDecoder().decode(data));
		});

		vm.writeShell(shellId, "hello-flat-shell\n");

		// Wait for output to arrive
		await new Promise((r) => setTimeout(r, 1000));

		vm.closeShell(shellId);

		const output = chunks.join("");
		expect(output).toContain("hello-flat-shell");
	}, 30_000);
});
