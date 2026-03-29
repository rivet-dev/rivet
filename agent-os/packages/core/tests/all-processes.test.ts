import { afterEach, beforeEach, describe, expect, test } from "vitest";
import { AgentOs } from "../src/agent-os.js";

describe("allProcesses()", () => {
	let vm: AgentOs;

	beforeEach(async () => {
		vm = await AgentOs.create();
	});

	afterEach(async () => {
		await vm.dispose();
	});

	test("returns empty on a fresh VM with no spawned processes", () => {
		const all = vm.allProcesses();
		expect(all).toEqual([]);
	});

	test("spawned process appears in allProcesses alongside kernel processes", async () => {
		const before = vm.allProcesses();
		await vm.writeFile("/tmp/stay.mjs", "setTimeout(() => {}, 30000);");
		const { pid } = vm.spawn("node", ["/tmp/stay.mjs"], {
			env: { HOME: "/home/user" },
		});

		const after = vm.allProcesses();
		expect(after.length).toBeGreaterThan(before.length);

		const found = after.find((p) => p.pid === pid);
		expect(found).toBeDefined();
		expect(found?.command).toBe("node");

		vm.killProcess(pid);
	}, 30_000);

	test("ppid relationships are correct", async () => {
		await vm.writeFile("/tmp/child.mjs", "setTimeout(() => {}, 30000);");
		const { pid } = vm.spawn("node", ["/tmp/child.mjs"], {
			env: { HOME: "/home/user" },
		});

		const all = vm.allProcesses();
		const child = all.find((p) => p.pid === pid);
		expect(child).toBeDefined();
		// ppid should reference an existing process (the kernel init or similar)
		expect(child?.ppid).toBeGreaterThanOrEqual(0);
		if (child?.ppid > 0) {
			const parent = all.find((p) => p.pid === child?.ppid);
			expect(parent).toBeDefined();
		}

		vm.killProcess(pid);
	}, 30_000);
});
