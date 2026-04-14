import { describe, expect, test } from "vitest";
import { setup } from "@/mod";
import { setupTest } from "@/test/mod";
import { sandboxActor } from "./index";
import type { SandboxProvider } from "sandbox-agent";

describe("sandbox actor direct URL access", () => {
	test("getSandboxUrl provisions the sandbox without connecting the SDK", async (c) => {
		let createCalls = 0;
		let destroyCalls = 0;
		let getUrlCalls = 0;

		const provider: SandboxProvider = {
			name: "test",
			async create() {
				createCalls += 1;
				return "sandbox-1";
			},
			async destroy() {
				destroyCalls += 1;
			},
			async getUrl(sandboxId) {
				getUrlCalls += 1;
				return `https://sandbox.example/${sandboxId}`;
			},
		};

		const registry = setup({
			use: {
				sandbox: sandboxActor({
					provider,
				}),
			},
		});
		const { client } = await setupTest(c, registry);
		const sandbox = client.sandbox.getOrCreate(["task-1"]);

		const result = await sandbox.getSandboxUrl();
		expect(result.url).toMatch(/^https:\/\/sandbox\.example\//);
		expect(createCalls).toBe(1);
		expect(getUrlCalls).toBe(1);

		await sandbox.destroy();
		expect(destroyCalls).toBe(1);
		await expect(sandbox.getSandboxUrl()).rejects.toThrow(
			"Internal error. Read the server logs for more details.",
		);
	});
});
