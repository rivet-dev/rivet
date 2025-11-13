import { resolve } from "node:path";
import tsconfigPaths from "vite-tsconfig-paths";
import { defineConfig } from "vitest/config";
import defaultConfig from "../../../vitest.base.ts";

export default defineConfig({
	...defaultConfig,
	// Used to resolve "rivetkit" to "src/mod.ts" in the test fixtures
	plugins: [tsconfigPaths()],
});
