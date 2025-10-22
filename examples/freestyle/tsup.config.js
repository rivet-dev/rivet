import { defineConfig } from "tsup";

export default defineConfig({
	entry: ["src/backend/server.ts"],
	outDir: "dist",
	// Vite build is in dist/public/* so we don't want to clean it
	clean: false,
	bundle: true,
	platform: "node",
	target: "deno2.5",
	external: [],
	// Include rivetkit in the bundle since it's local
	// Note: this makes bundle much larger and can be
	// removed if using a version of rivetkit from npm
	noExternal: ["rivetkit", "hono"],
});
