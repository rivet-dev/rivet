import { defineConfig } from "tsup";

export default defineConfig({
	entry: ["src/index.ts", "src/cli.ts"],
	format: ["esm"],
	dts: true,
	sourcemap: true,
	clean: true,
	target: "node20",
});
