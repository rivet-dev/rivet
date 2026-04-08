import type { SandboxProvider } from "sandbox-agent";
import { describe, expect, test, vi } from "vitest";
import { setup } from "@/mod";
import { setupTest } from "@/test/mod";
import { sandboxActor } from "./index";

describe("sandbox actor direct URL access", () => {
	test("getSandboxUrl provisions the sandbox without connecting the SDK", async (c) => {
		const provider: SandboxProvider = {
			name: "test",
			create: vi.fn(async () => "sandbox-1"),
			destroy: vi.fn(async () => {}),
			getUrl: vi.fn(
				async (sandboxId) => `https://sandbox.example/${sandboxId}`,
			),
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
		expect(provider.create).toHaveBeenCalledTimes(1);
		expect(provider.getUrl).toHaveBeenCalled();

		await sandbox.destroy();
		await expect(sandbox.getSandboxUrl()).rejects.toThrow(
			"Internal error. Read the server logs for more details.",
		);
	});
});
