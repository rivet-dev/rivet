import * as crypto from "node:crypto";
import path from "node:path";
import { sentryVitePlugin } from "@sentry/vite-plugin";
import { tanstackRouter } from "@tanstack/router-plugin/vite";
import react from "@vitejs/plugin-react";
import { defineConfig, loadEnv } from "vite";
import tsconfigPaths from "vite-tsconfig-paths";

// These are only needed in CI. They'll be undefined in dev.
const GIT_BRANCH = process.env.CF_PAGES_BRANCH;
const GIT_SHA = process.env.CF_PAGES_COMMIT_SHA;

// https://vitejs.dev/config/
export default defineConfig(({ mode }) => {
	const env = loadEnv(mode, process.cwd(), "");

	return {
		base: "/ui",
		plugins: [
			tsconfigPaths(),
			react(),
			tanstackRouter(),
			env.SENTRY_AUTH_TOKEN
				? sentryVitePlugin({
						org: "rivet-gaming",
						project: "hub",
						authToken: env.SENTRY_AUTH_TOKEN,
						release:
							GIT_BRANCH === "main"
								? { name: GIT_SHA }
								: undefined,
					})
				: null,
		],
		server: {
			port: 43708,
			proxy: {
				"/api": {
					target: "http://localhost:6420",
					changeOrigin: true,
					rewrite: (path) => path.replace(/^\/api/, ""),
				},
			},
		},
		preview: {
			port: 43708,
		},
		define: {
			// Provide a unique build ID for cache busting
			__APP_TYPE__: JSON.stringify(env.APP_TYPE || "engine"),
			__APP_BUILD_ID__: JSON.stringify(
				`${new Date().toISOString()}@${crypto.randomUUID()}`,
			),
		},
		resolve: {
			alias: {
				"@": path.resolve(__dirname, "./src"),
			},
		},
		build: {
			sourcemap: true,
			commonjsOptions: {
				include: [/@rivet-gg\/components/, /node_modules/],
			},
		},
		optimizeDeps: {
			include: ["@fortawesome/*", "@rivet-gg/icons"],
		},
		worker: {
			format: "es",
		},
	};
});
