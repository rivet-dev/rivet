import { defineConfig } from "tsup";

export default defineConfig({
	entry: ["src/mod.ts"],
	format: ["esm", "cjs"],
	dts: true,
	clean: true,
});
