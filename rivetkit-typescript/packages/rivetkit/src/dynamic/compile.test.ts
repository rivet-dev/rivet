import { describe, expect, test } from "vitest";
import { compileActorSource } from "./compile";

// @secure-exec/typescript and secure-exec are optional peer dependencies.
// Skip the suite when they are not installed.
let secureExecAvailable = false;
try {
	require.resolve("secure-exec");
	require.resolve("@secure-exec/typescript");
	secureExecAvailable = true;
} catch {
	// Not available.
}

describe.skipIf(!secureExecAvailable)(
	"compileActorSource",
	() => {
		test("valid TypeScript returns JS and success: true", async () => {
			const result = await compileActorSource({
				source: `
					const greeting: string = "hello";
					export default greeting;
				`,
				typecheck: false,
			});

			expect(result.success).toBe(true);
			expect(result.js).toBeDefined();
			expect(result.js).toContain("hello");
			expect(result.diagnostics.length).toBe(0);
		});

		test("TypeScript with type errors returns diagnostics and success: false when typecheck is true", async () => {
			const result = await compileActorSource({
				source: `
					const greeting: number = "hello";
					export default greeting;
				`,
				typecheck: true,
			});

			expect(result.success).toBe(false);
			expect(result.diagnostics.length).toBeGreaterThan(0);
			expect(
				result.diagnostics.some((d) => d.category === "error"),
			).toBe(true);
		});

		test("typecheck: false strips types without error on invalid types", async () => {
			const result = await compileActorSource({
				source: `
					const greeting: number = "hello";
					export default greeting;
				`,
				typecheck: false,
			});

			expect(result.success).toBe(true);
			expect(result.js).toBeDefined();
			expect(result.js).toContain("hello");
		});
	},
);
