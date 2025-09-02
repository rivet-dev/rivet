import type { Options } from "tsup";

export default {
	target: "node16",
	platform: "node",
	format: ["cjs", "esm"],
	sourcemap: true,
	clean: true,
	dts: {
		compilerOptions: {
			skipLibCheck: true,
			resolveJsonModule: true,
		},
	},
	minify: false,
	// IMPORTANT: Splitting is required to fix a bug with ESM (https://github.com/egoist/tsup/issues/992#issuecomment-1763540165)
	splitting: true,
	skipNodeModulesBundle: true,
	publicDir: true,
} satisfies Options;
