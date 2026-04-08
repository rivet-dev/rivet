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
				const testDir = `/home/sandbox/tmp-${crypto.randomUUID()}`;
				const testFile = `${testDir}/hello.txt`;
				const renamedFile = `${testDir}/renamed.txt`;
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

				await sandbox.mkdirFs({ path: testDir });
				await sandbox.writeFsFile(
					{ path: testFile },
					"sandbox actor driver test",
				);
				expect(
					decoder.decode(
						await sandbox.readFsFile({
							path: testFile,
						}),
					),
				).toBe("sandbox actor driver test");

				const stat = await sandbox.statFs({
					path: testFile,
				});
				expect(stat.entryType).toBe("file");

				await sandbox.moveFs({
					from: testFile,
					to: renamedFile,
				});
				expect(
					(await sandbox.listFsEntries({ path: testDir })).map(
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
							path: renamedFile,
						}),
					),
				).toBe("sandbox actor driver test");

				await sandbox.deleteFsEntry({
					path: testDir,
					recursive: true,
				});
				expect(
					await sandbox.listFsEntries({ path: "/home/sandbox" }),
				).not.toEqual(
					expect.arrayContaining([
						expect.objectContaining({ name: testDir.split("/").at(-1) }),
					]),
				);
			}, 180_000);
		},
	);
}
