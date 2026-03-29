import { afterEach, beforeEach, describe, expect, test } from "vitest";
import { AgentOs } from "../src/agent-os.js";

describe("listAgents()", () => {
	let vm: AgentOs;

	beforeEach(async () => {
		vm = await AgentOs.create();
	});

	afterEach(async () => {
		await vm.dispose();
	});

	test("returns pi and opencode agents", () => {
		const agents = vm.listAgents();
		const ids = agents.map((a) => a.id);
		expect(ids).toContain("pi");
		expect(ids).toContain("opencode");
	});

	test("each entry has correct fields from AGENT_CONFIGS", () => {
		const agents = vm.listAgents();
		const pi = agents.find((a) => a.id === "pi");
		expect(pi).toBeDefined();
		expect(pi?.acpAdapter).toBe("pi-acp");
		expect(pi?.agentPackage).toBe("@mariozechner/pi-coding-agent");
		expect(typeof pi?.installed).toBe("boolean");
	});

	test("installed is true when adapter package exists", () => {
		// pi-acp should be installed in node_modules (used by the project)
		const agents = vm.listAgents();
		const pi = agents.find((a) => a.id === "pi");
		expect(pi?.installed).toBe(true);
	});

	test("installed is false when adapter package is missing", async () => {
		// Create a VM with moduleAccessCwd pointing to a directory without node_modules
		const vm2 = await AgentOs.create({ moduleAccessCwd: "/tmp" });
		try {
			const agents = vm2.listAgents();
			// No packages installed in /tmp
			for (const agent of agents) {
				expect(agent.installed).toBe(false);
			}
		} finally {
			await vm2.dispose();
		}
	});
});
