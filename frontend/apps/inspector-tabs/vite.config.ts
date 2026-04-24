import * as crypto from "node:crypto";
import path from "node:path";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";
import tsconfigPaths from "vite-tsconfig-paths";

const TABS = [
	"workflow",
	"database",
	"state",
	"queue",
	"connections",
	"metadata",
] as const;

// https://vitejs.dev/config/
export default defineConfig({
	root: path.resolve(__dirname),
	base: "/ui/tabs/",
	publicDir: path.resolve(__dirname, "../../public"),
	envDir: path.resolve(__dirname, "../.."),
	plugins: [react(), tsconfigPaths()],
	define: {
		__APP_TYPE__: JSON.stringify("inspector"),
		__APP_BUILD_ID__: JSON.stringify(
			`${new Date().toISOString()}@${crypto.randomUUID()}`,
		),
	},
	optimizeDeps: {
		include: ["@fortawesome/*", "@rivet-gg/icons", "@rivet-gg/cloud"],
	},
	worker: {
		format: "es",
	},
	build: {
		outDir: "../../dist/inspector/tabs",
		sourcemap: true,
		rollupOptions: {
			input: Object.fromEntries(
				TABS.map((tab) => [
					tab,
					path.resolve(__dirname, `entries/${tab}/index.html`),
				]),
			),
		},
		commonjsOptions: {
			include: [/@rivet-gg\/components/, /node_modules/],
		},
	},
});
