import { afterEach, beforeEach, describe, expect, test } from "vitest";
import { AgentOs } from "../src/agent-os.js";

describe("processTree()", () => {
	let vm: AgentOs;

	beforeEach(async () => {
		vm = await AgentOs.create();
	});

	afterEach(async () => {
		await vm.dispose();
	});

	test("returns empty array on fresh VM", () => {
		expect(vm.processTree()).toEqual([]);
	});

	test("spawned process appears as a root in the tree", async () => {
		await vm.writeFile("/tmp/stay.mjs", "setTimeout(() => {}, 30000);");
		const { pid } = vm.spawn("node", ["/tmp/stay.mjs"], {
			env: { HOME: "/home/user" },
		});

		const tree = vm.processTree();
		// The node process should be a root (ppid 0 or orphan)
		const root = tree.find((n) => n.pid === pid);
		expect(root).toBeDefined();
		expect(root?.children).toEqual([]);

		vm.killProcess(pid);
	}, 30_000);

	test("parent-child tree structure: node script spawning a child", async () => {
		// Parent spawns a child process via child_process.spawn
		await vm.writeFile(
			"/tmp/parent.mjs",
			`
import { spawn } from "node:child_process";
const child = spawn("node", ["/tmp/child.mjs"]);
// Keep parent alive
setTimeout(() => {}, 30000);
`,
		);
		await vm.writeFile("/tmp/child.mjs", "setTimeout(() => {}, 30000);");

		const { pid } = vm.spawn("node", ["/tmp/parent.mjs"], {
			env: { HOME: "/home/user" },
		});

		// Give it a moment for the child to spawn
		await new Promise((r) => setTimeout(r, 1000));

		const tree = vm.processTree();
		const parentNode = tree.find((n) => n.pid === pid);
		expect(parentNode).toBeDefined();
		expect(parentNode?.children.length).toBeGreaterThanOrEqual(1);

		// Child's ppid should point to parent
		const childNode = parentNode?.children[0];
		expect(childNode.ppid).toBe(pid);

		vm.killProcess(pid);
	}, 30_000);
});
