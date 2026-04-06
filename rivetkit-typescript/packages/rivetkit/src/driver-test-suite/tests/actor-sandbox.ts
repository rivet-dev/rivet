// @ts-nocheck
import { describe, expect, test, vi } from "vitest";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest } from "../utils";

export function runActorSandboxTests(driverTestConfig: DriverTestConfig) {
	describe.skipIf(driverTestConfig.skip?.sandbox)(
		"Actor Sandbox Tests",
		() => {
			test("supports sandbox actions through the actor runtime", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const sandbox = client.dockerSandboxActor.getOrCreate([
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
						await sandbox.readFsFile({
							path: "/root/tmp/hello.txt",
						}),
					),
				).toBe("sandbox actor driver test");

				const stat = await sandbox.statFs({
					path: "/root/tmp/hello.txt",
				});
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
						await sandbox.readFsFile({
							path: "/root/tmp/renamed.txt",
						}),
					),
				).toBe("sandbox actor driver test");

				await sandbox.deleteFsEntry({
					path: "/root/tmp",
					recursive: true,
				});
				expect(
					await sandbox.listFsEntries({ path: "/root" }),
				).not.toEqual(
					expect.arrayContaining([
						expect.objectContaining({ name: "tmp" }),
					]),
				);
			}, 180_000);
		},
	);
}
