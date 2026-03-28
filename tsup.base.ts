import type { Options } from "tsup";

export default {
	target: "node16",
	platform: "node",
	format: ["cjs", "esm"],
	sourcemap: true,
	clean: true,
	// DTS is generated separately via tsc (see tsconfig.build.json per package)
	dts: false,
	minify: false,
	// IMPORTANT: Splitting is required to fix a bug with ESM (https://github.com/egoist/tsup/issues/992#issuecomment-1763540165)
	splitting: true,
	skipNodeModulesBundle: true,
	publicDir: true,
	external: [/^node:.*/],
	// Required to replace `import.meta.ur.` with CJS shims
	shims: true,
} satisfies Options;
