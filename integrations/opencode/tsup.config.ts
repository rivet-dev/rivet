import { defineConfig } from "tsup";

export default defineConfig({
	entry: ["src/index.ts"],
	format: ["esm"],
	platform: "node",
	target: "node22",
	dts: true,
	clean: true,
	sourcemap: true,
	// The preview SDK publishes extensionless internal imports. Bundle its entry
	// points so the published Rivet integration runs in plain Node.js.
	noExternal: [/^@opencode\/(?!util(?:\/|$))/, /^@parcel\/watcher\/wrapper$/],
	shims: true,
	removeNodeProtocol: false,
	// Some upstream dependencies load Node built-ins through CommonJS, including
	// OpenCode's code-mode TypeScript compiler. Vitest alone masks this requirement.
	banner: {
		js: 'import { createRequire as __rivetCreateRequire } from "node:module"; const require = __rivetCreateRequire(import.meta.url);',
	},
	esbuildPlugins: [
		{
			name: "opencode-native-assets",
			setup(build) {
				// These providers resolve native binaries/WASM relative to their own
				// package. Preserve that location instead of relocating their assets.
				const providers: Record<string, string> = {
					"#fff": "filesystem/fff.node",
					"#pty": "pty/pty.node",
					"#shell-parser-wasm": "shell/parser-wasm.node",
					"#photon-wasm": "image/photon-wasm.node",
					"#persistent-pty-binary": "persistent-pty/binary.node",
					"#process-lock-ffi": "util/process-lock-ffi.node",
				};
				build.onResolve({ filter: /^#/ }, ({ path }) =>
					providers[path]
						? { path: `@opencode/core/${providers[path]}`, external: true }
						: undefined,
				);
			},
		},
	],
});
