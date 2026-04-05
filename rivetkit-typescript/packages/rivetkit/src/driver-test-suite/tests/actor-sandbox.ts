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
				const baseDir = `/tmp/rivetkit-sandbox-${crypto.randomUUID()}`;
				const helloPath = `${baseDir}/hello.txt`;
				const renamedPath = `${baseDir}/renamed.txt`;
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

				await sandbox.mkdirFs({ path: baseDir });
				await sandbox.writeFsFile(
					{ path: helloPath },
					"sandbox actor driver test",
				);
				expect(
					decoder.decode(
						await sandbox.readFsFile({
							path: helloPath,
						}),
					),
				).toBe("sandbox actor driver test");

				const stat = await sandbox.statFs({
					path: helloPath,
				});
				expect(stat.entryType).toBe("file");

				await sandbox.moveFs({
					from: helloPath,
					to: renamedPath,
				});
				expect(
					(await sandbox.listFsEntries({ path: baseDir })).map(
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
							path: renamedPath,
						}),
					),
				).toBe("sandbox actor driver test");

				await sandbox.deleteFsEntry({
					path: baseDir,
					recursive: true,
				});
				expect(
					await sandbox.listFsEntries({ path: "/tmp" }),
				).not.toEqual(
					expect.arrayContaining([
						expect.objectContaining({
							name: baseDir.split("/").at(-1),
						}),
					]),
				);

				await sandbox.destroy();
			}, 180_000);
		},
	);
}
