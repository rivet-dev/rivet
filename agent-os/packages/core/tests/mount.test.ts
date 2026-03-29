import { afterEach, describe, expect, test } from "vitest";
import { AgentOs, createInMemoryFileSystem } from "../src/index.js";

describe("mount integration", () => {
	let vm: AgentOs;

	afterEach(async () => {
		await vm.dispose();
	});

	test("create with memory mount", async () => {
		vm = await AgentOs.create({
			mounts: [{ path: "/data", driver: createInMemoryFileSystem() }],
		});
		expect(await vm.exists("/data")).toBe(true);
	});

	test("writeFile and readFile round-trip through mounted backend", async () => {
		vm = await AgentOs.create({
			mounts: [{ path: "/data", driver: createInMemoryFileSystem() }],
		});
		await vm.writeFile("/data/foo.txt", "hello mount");
		const data = await vm.readFile("/data/foo.txt");
		expect(new TextDecoder().decode(data)).toBe("hello mount");
	});

	test("root FS and mount are separate", async () => {
		vm = await AgentOs.create({
			mounts: [{ path: "/data", driver: createInMemoryFileSystem() }],
		});
		await vm.writeFile("/home/user/foo.txt", "root content");
		await vm.writeFile("/data/foo.txt", "mount content");

		const rootData = await vm.readFile("/home/user/foo.txt");
		const mountData = await vm.readFile("/data/foo.txt");
		expect(new TextDecoder().decode(rootData)).toBe("root content");
		expect(new TextDecoder().decode(mountData)).toBe("mount content");
	});

	test("runtime mountFs and unmountFs work", async () => {
		vm = await AgentOs.create();

		vm.mountFs("/mnt/dynamic", createInMemoryFileSystem());
		await vm.writeFile("/mnt/dynamic/test.txt", "dynamic");
		const data = await vm.readFile("/mnt/dynamic/test.txt");
		expect(new TextDecoder().decode(data)).toBe("dynamic");

		vm.unmountFs("/mnt/dynamic");
		await expect(vm.readFile("/mnt/dynamic/test.txt")).rejects.toThrow();
	});

	test("readdir('/') includes 'data' alongside standard POSIX dirs", async () => {
		vm = await AgentOs.create({
			mounts: [{ path: "/data", driver: createInMemoryFileSystem() }],
		});
		const entries = await vm.readdir("/");
		expect(entries).toContain("data");
		// Standard POSIX dirs should also be present
		expect(entries).toContain("tmp");
		expect(entries).toContain("home");
	});

	test("rename across mounts throws EXDEV", async () => {
		vm = await AgentOs.create({
			mounts: [{ path: "/data", driver: createInMemoryFileSystem() }],
		});
		await vm.writeFile("/data/cross.txt", "cross-mount");
		await expect(
			vm.move("/data/cross.txt", "/home/user/cross.txt"),
		).rejects.toThrow("EXDEV");
	});

	test("readOnly mount blocks writeFile with EROFS", async () => {
		vm = await AgentOs.create({
			mounts: [{ path: "/ro", driver: createInMemoryFileSystem(), readOnly: true }],
		});
		await expect(
			vm.writeFile("/ro/blocked.txt", "should fail"),
		).rejects.toThrow("EROFS");
	});
});
