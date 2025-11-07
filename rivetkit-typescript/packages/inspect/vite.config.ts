import path from "node:path";
import { tanstackRouter } from "@tanstack/router-plugin/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// https://vite.dev/config/
export default defineConfig({
	plugins: [
		tanstackRouter({ target: "react", autoCodeSplitting: true }),
		react(),
	],
	optimizeDeps: {
		include: ["@fortawesome/*", "@rivet-gg/icons"],
	},
	define: {
		__APP_TYPE__: JSON.stringify("inspector"),
		__APP_BUILD_ID__: JSON.stringify(Date.now().toString()),
	},
	build: {
		lib: {
			entry: path.resolve(__dirname, "src/main.tsx"),
			name: "Inspector",
			// the proper extensions will be added
			fileName: "index",
		},
	},
	resolve: {
		alias: {
			"@": path.resolve(__dirname, "src"),
		},
	},
	worker: {
		format: "es",
	},
});
