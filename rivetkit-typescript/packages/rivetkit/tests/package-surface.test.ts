import { describe, expect, test } from "vitest";
import packageJson from "../package.json" with { type: "json" };

describe("package surface", () => {
	test("does not advertise deleted topology entrypoints", () => {
		expect(packageJson.exports).not.toHaveProperty(
			"./topologies/coordinate",
		);
		expect(packageJson.exports).not.toHaveProperty(
			"./topologies/partition",
		);
		expect(packageJson.scripts.build).not.toContain("src/topologies/");
	});

	test("does not keep obviously dead package metadata", () => {
		expect(packageJson.files).not.toContain("deno.json");
		expect(packageJson.files).not.toContain("bun.json");

		expect(packageJson.dependencies).not.toHaveProperty(
			"@hono/standard-validator",
		);
		expect(packageJson.dependencies).not.toHaveProperty(
			"@rivetkit/fast-json-patch",
		);
		expect(packageJson.dependencies).not.toHaveProperty(
			"@rivetkit/on-change",
		);
		expect(packageJson.dependencies).not.toHaveProperty("nanoevents");

		expect(packageJson.devDependencies).not.toHaveProperty("@types/ws");
		expect(packageJson.devDependencies).not.toHaveProperty("@vitest/ui");
		expect(packageJson.devDependencies).not.toHaveProperty("cli-table3");
		expect(packageJson.devDependencies).not.toHaveProperty("commander");
		expect(packageJson.devDependencies).not.toHaveProperty("local-pkg");
		expect(packageJson.devDependencies).not.toHaveProperty(
			"zod-to-json-schema",
		);
	});
});
