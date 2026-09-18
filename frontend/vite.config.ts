import { sentryVitePlugin } from "@sentry/vite-plugin";
import { tanstackRouter } from "@tanstack/router-plugin/vite";
import react from "@vitejs/plugin-react";
import favigo from "favigo/vite";
import Macros from "unplugin-macros/vite";
import { defineConfig, loadEnv, mergeConfig } from "vite";
import { commonEnvSchema } from "./src/lib/env";
import { baseViteConfig } from "./vite.base.config";

// These are only needed in CI. They'll be undefined in dev.
const GIT_BRANCH = process.env.RAILWAY_GIT_BRANCH;
const GIT_SHA = process.env.RAILWAY_GIT_COMMIT_SHA;

const getVariantForMode = (mode: string) => {
	switch (mode) {
		case "staging":
			return {
				type: "badge",
				text: "DEV",
				backgroundColor: "#FF4F00",
				textColor: "#ffffff",
				position: "bottom-right",
				size: "large",
			} as const;
		default:
			return undefined;
	}
};

function isFlagEnabled(
	featureFlags: string | undefined,
	flag: string,
): boolean {
	if (featureFlags === undefined) return true;
	return featureFlags
		.split(",")
		.map((s) => s.trim())
		.includes(flag);
}

// https://vitejs.dev/config/
export default defineConfig(({ mode }) => {
	const env = commonEnvSchema.parse(loadEnv(mode, process.cwd(), ""));
	const featureFlags = process.env.VITE_FEATURE_FLAGS;
	const supportEnabled = isFlagEnabled(featureFlags, "support");
	const multitenancyEnabled = isFlagEnabled(featureFlags, "multitenancy");
	const base = multitenancyEnabled ? "/" : "/ui/";

	console.log(
		env.SENTRY_AUTH_TOKEN
			? "Sentry plugin enabled"
			: "Sentry plugin disabled (missing auth token)",
	);

	return mergeConfig(baseViteConfig(), {
		base,
		plugins: [
			tanstackRouter({ target: "react", autoCodeSplitting: true }),
			react(),
			env.SENTRY_AUTH_TOKEN
				? sentryVitePlugin({
						org: "rivet-gaming",
						project: env.SENTRY_PROJECT,
						authToken: env.SENTRY_AUTH_TOKEN,
						release:
							GIT_BRANCH === "main"
								? { name: GIT_SHA }
								: undefined,
						// The plugin logs upload failures and lets the build
						// succeed, which ships a bundle whose stack traces never
						// symbolicate. Build without a token to opt out entirely.
						errorHandler: (error) => {
							throw error;
						},
						sourcemaps: {
							// Caddy serves everything under dist/, so maps left
							// behind are public. Debug IDs in the bundle are what
							// Sentry matches on, not the served maps.
							filesToDeleteAfterUpload: ["./dist/**/*.map"],
						},
					})
				: null,
			favigo({
				source: "./public/favicon.svg",
				variant: getVariantForMode(
					env.VITE_DEPLOYMENT_TYPE || "production",
				),
				configuration: {
					path: env.BASE_URL ?? "/",
					theme_color: "#FF4F00",
					background: "transparent",
				},
			}),
			Macros(),
		],
		server: {
			port: 43708,
			proxy: {
				"/api": {
					target: "http://localhost:6420",
					changeOrigin: true,
					rewrite: (path: string) => path.replace(/^\/api/, ""),
				},
			},
			// Accept the shared dev tunnel hostname.
			// See docs-internal/platform/dev-tunnel.md.
			// allowedHosts: ["dashboard.dev.rivet.dev"],
			allowedHosts: ["local.staging.rivet.dev"],
		},
		preview: {
			port: 43708,
		},
		build: {
			// Sourcemaps: on in dev, and in prod only when a Sentry auth token is
			// present (so they can be uploaded to Sentry). Otherwise off, since the
			// engine binary embeds frontend/dist via include_dir! and shipping maps
			// bakes tens of MB of debug data into the engine-cli binary.
			sourcemap: mode !== "production" || Boolean(env.SENTRY_AUTH_TOKEN),
			commonjsOptions: {
				include: [/@rivet-gg\/components/, /node_modules/],
			},
		},
	});
});
