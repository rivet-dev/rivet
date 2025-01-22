import { defineConfig } from "tsup";

export default defineConfig({
	target: "es2020",
	format: ["cjs", "esm"],
	sourcemap: true,
	clean: true,
	dts: true,
	external: ["react", "react-dom", "@kentcdodds/tmp-react-server-dom-esm"],
	// Bundle only the local dependencies
	noExternal: [/@rivet-gg\/.*?/],
	minify: true,
	platform: "node",
});
