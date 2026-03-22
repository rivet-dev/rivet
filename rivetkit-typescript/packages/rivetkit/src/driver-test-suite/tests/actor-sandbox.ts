import { describe, expect, test, vi } from "vitest";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest } from "../utils";

function sandboxTestSuite(
	driverTestConfig: DriverTestConfig,
	actorName: string,
	label: string,
) {
	describe.skipIf(driverTestConfig.skip?.sandbox)(`Actor Sandbox Tests (${label})`, () => {
		test("supports sandbox actions through the actor runtime", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const sandbox = (client as any)[actorName].getOrCreate([
				`sandbox-${crypto.randomUUID()}`,
			]);
			const decoder = new TextDecoder();

			const health = await vi.waitFor(
				async () => {
					return await sandbox.getHealth();
				},
				{
					timeout: 120_000,
					interval: 500,
				},
			);
			expect(typeof health.status).toBe("string");
			const { url } = await sandbox.getSandboxUrl();
			expect(url).toMatch(/^https?:\/\//);

			await sandbox.mkdirFs({ path: "/root/tmp" });
			await sandbox.writeFsFile(
				{ path: "/root/tmp/hello.txt" },
				"sandbox actor driver test",
			);
			expect(
				decoder.decode(
					await sandbox.readFsFile({ path: "/root/tmp/hello.txt" }),
				),
			).toBe("sandbox actor driver test");

			const stat = await sandbox.statFs({ path: "/root/tmp/hello.txt" });
			expect(stat.entryType).toBe("file");

				await sandbox.moveFs({
					from: "/root/tmp/hello.txt",
					to: "/root/tmp/renamed.txt",
				});
				expect(
					(await sandbox.listFsEntries({ path: "/root/tmp" })).map(
						(entry: { name: string }) => entry.name,
					),
				).toContain("renamed.txt");

			await sandbox.dispose();

			const healthAfterDispose = await vi.waitFor(
				async () => {
					return await sandbox.getHealth();
				},
				{
					timeout: 120_000,
					interval: 500,
				},
			);
			expect(typeof healthAfterDispose.status).toBe("string");
			expect(
				decoder.decode(
					await sandbox.readFsFile({ path: "/root/tmp/renamed.txt" }),
				),
			).toBe("sandbox actor driver test");

			await sandbox.deleteFsEntry({ path: "/root/tmp", recursive: true });
			expect(await sandbox.listFsEntries({ path: "/root" })).not.toEqual(
				expect.arrayContaining([
					expect.objectContaining({ name: "tmp" }),
				]),
			);
		}, 180_000);
	});
}

export function runActorSandboxTests(driverTestConfig: DriverTestConfig) {
	sandboxTestSuite(driverTestConfig, "dockerSandboxActor", "Docker");

	// E2B tests only run when E2B_API_KEY is available.
	const hasE2b = !!process.env.E2B_API_KEY;
	describe.skipIf(!hasE2b || driverTestConfig.skip?.sandbox)(
		"Actor Sandbox Tests (E2B)",
		() => {
			test("creates sandbox and passes health check", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const sandbox = (client as any).e2bSandboxActor.getOrCreate([
					`sandbox-e2b-${crypto.randomUUID()}`,
				]);

				const health = await vi.waitFor(
					async () => {
						return await sandbox.getHealth();
					},
					{
						timeout: 120_000,
						interval: 500,
					},
				);
				expect(typeof health.status).toBe("string");
			}, 180_000);

			test("supports file system operations", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const sandbox = (client as any).e2bSandboxActor.getOrCreate([
					`sandbox-e2b-${crypto.randomUUID()}`,
				]);
				const decoder = new TextDecoder();

				await vi.waitFor(
					async () => {
						return await sandbox.getHealth();
					},
					{
						timeout: 120_000,
						interval: 500,
					},
				);

				await sandbox.mkdirFs({ path: "/root/tmp" });
				await sandbox.writeFsFile(
					{ path: "/root/tmp/hello.txt" },
					"e2b sandbox actor driver test",
				);
				expect(
					decoder.decode(
						await sandbox.readFsFile({ path: "/root/tmp/hello.txt" }),
					),
				).toBe("e2b sandbox actor driver test");

				const stat = await sandbox.statFs({ path: "/root/tmp/hello.txt" });
				expect(stat.entryType).toBe("file");

				await sandbox.deleteFsEntry({ path: "/root/tmp", recursive: true });
			}, 180_000);
		},
	);
}
