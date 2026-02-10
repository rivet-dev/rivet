import { sentryVitePlugin } from "@sentry/vite-plugin";
import { tanstackRouter } from "@tanstack/router-plugin/vite";
import react from "@vitejs/plugin-react";
import favigo from "favigo/vite";
import Macros from "unplugin-macros/vite";
import { defineConfig, loadEnv, mergeConfig, type Plugin } from "vite";
import { commonEnvSchema } from "./src/lib/env";
import { baseViteConfig } from "./vite.base.config";

// These are only needed in CI. They'll be undefined in dev.
const GIT_BRANCH = process.env.CF_PAGES_BRANCH;
const GIT_SHA = process.env.CF_PAGES_COMMIT_SHA;

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

// https://vitejs.dev/config/
export default defineConfig(({ mode }) => {
	const env = commonEnvSchema.parse(loadEnv(mode, process.cwd(), ""));

	console.log(
		env.SENTRY_AUTH_TOKEN
			? "Sentry plugin enabled"
			: "Sentry plugin disabled (missing auth token)",
	);

	return mergeConfig(baseViteConfig(), {
		base: "/ui",
		plugins: [
			tanstackRouter({ target: "react", autoCodeSplitting: true }),
			react(),
			liveChatPlugin(),
			env.SENTRY_AUTH_TOKEN
				? sentryVitePlugin({
						org: "rivet-gaming",
						project: env.SENTRY_PROJECT,
						authToken: env.SENTRY_AUTH_TOKEN,
						release:
							GIT_BRANCH === "main"
								? { name: GIT_SHA }
								: undefined,
					})
				: null,
			favigo({
				source: "./public/favicon.svg",
				variant: getVariantForMode(env.DEPLOYMENT_TYPE || "production"),
				configuration: {
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
		},
		preview: {
			port: 43708,
		},
		define: {
			__APP_TYPE__: JSON.stringify(env.APP_TYPE || "engine"),
		},
		build: {
			sourcemap: true,
			commonjsOptions: {
				include: [/@rivet-gg\/components/, /node_modules/],
			},
		},
	});
});

export function liveChatPlugin(source: string = ""): Plugin {
	return {
		name: "live-chat-plugin",
		transformIndexHtml(html) {
			return html.replace(/{{live_chat}}/, source);
		},
	};
}
