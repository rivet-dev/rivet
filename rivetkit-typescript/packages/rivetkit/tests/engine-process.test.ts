import { getPlatformPackageName } from "@rivetkit/engine-cli";
import { describe, expect, it } from "vitest";

// More exhaustive platform-mapping tests live in @rivetkit/engine-cli itself.
// Here we just sanity-check that a valid name comes back for the host.
describe("getPlatformPackageName", () => {
	it("returns a platform package name for the current host", () => {
		const name = getPlatformPackageName();
		if (process.platform === "linux" || process.platform === "darwin") {
			expect(name).toMatch(/^@rivetkit\/engine-cli-/);
		}
	});
});
