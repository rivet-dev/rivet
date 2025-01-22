import * as esbuild from "esbuild";
import { glob } from "glob";

// Find all mod.ts files in src/tasks directories
const entryPoints = await glob("src/tasks/*/mod.ts");

// Build configuration
await esbuild.build({
	entryPoints,
	outdir: "dist",
	bundle: true,
	sourcemap: true,
	platform: "node",
	format: "esm",
	target: "esnext",
	outbase: "./",
	plugins: [
		{
			name: "replace-esbuild",
			setup(build) {
				build.onResolve({ filter: /^esbuild$/ }, (args) => {
					return { path: "npm:esbuild@^0.20.2", external: true };
				});
			},
		},
	],
	banner: {
		js: 'import { createRequire } from "node:module";\nconst require = createRequire(import.meta.url);\nconst __filename = import.meta.filename;\nconst __dirname = import.meta.dirname;',
	},
});
