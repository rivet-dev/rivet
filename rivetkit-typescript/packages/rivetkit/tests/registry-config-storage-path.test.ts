import { RegistryConfigSchema } from "@/registry/config";
import { describe, expect, test } from "vitest";

describe.sequential("registry config storagePath", () => {
	test("reads storagePath from RIVETKIT_STORAGE_PATH when unset in config", () => {
		const previous = process.env.RIVETKIT_STORAGE_PATH;
		try {
			process.env.RIVETKIT_STORAGE_PATH = "/tmp/rivetkit-storage-env";
			const parsed = RegistryConfigSchema.parse({
				use: {},
			});

			expect(parsed.storagePath).toBe("/tmp/rivetkit-storage-env");
		} finally {
			if (previous === undefined) {
				delete process.env.RIVETKIT_STORAGE_PATH;
			} else {
				process.env.RIVETKIT_STORAGE_PATH = previous;
			}
		}
	});

	test("config storagePath overrides RIVETKIT_STORAGE_PATH", () => {
		const previous = process.env.RIVETKIT_STORAGE_PATH;
		try {
			process.env.RIVETKIT_STORAGE_PATH = "/tmp/rivetkit-storage-env";
			const parsed = RegistryConfigSchema.parse({
				use: {},
				storagePath: "/tmp/rivetkit-storage-config",
			});

			expect(parsed.storagePath).toBe("/tmp/rivetkit-storage-config");
		} finally {
			if (previous === undefined) {
				delete process.env.RIVETKIT_STORAGE_PATH;
			} else {
				process.env.RIVETKIT_STORAGE_PATH = previous;
			}
		}
	});
});
