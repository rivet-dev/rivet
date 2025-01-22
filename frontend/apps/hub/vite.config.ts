import { execSync } from "node:child_process";
import path from "node:path";
import { sentryVitePlugin } from "@sentry/vite-plugin";
import { transformerNotationFocus } from "@shikijs/transformers";
import { TanStackRouterVite } from "@tanstack/router-vite-plugin";
import react from "@vitejs/plugin-react";
import * as shiki from "shiki";
import { type Plugin, defineConfig } from "vite";
import vitePluginFaviconsInject from "vite-plugin-favicons-inject";

const GIT_BRANCH =
	process.env.CF_PAGES_BRANCH ||
	execSync("git rev-parse --abbrev-ref HEAD").toString().trim();

const GIT_SHA =
	process.env.CF_PAGES_COMMIT_SHA ||
	execSync("git rev-parse HEAD").toString().trim();

// https://vitejs.dev/config/
export default defineConfig({
	base: "./",
	plugins: [
		react(),
		TanStackRouterVite(),
		vitePluginFaviconsInject(
			path.resolve(__dirname, "public", "icon-white.svg"),
			{
				appName: "Rivet Hub",
				theme_color: "#ff4f00",
			},
		),
		shikiTransformer(),
		process.env.SENTRY_AUTH_TOKEN
			? sentryVitePlugin({
					org: "rivet-gaming",
					project: "hub",
					authToken: process.env.SENTRY_AUTH_TOKEN,
					release:
						GIT_BRANCH === "main" ? { name: GIT_SHA } : undefined,
				})
			: null,
	],
	server: {
		port: 5080,
	},
	define: {
		__APP_GIT_BRANCH__: JSON.stringify(GIT_BRANCH),
		__APP_GIT_COMMIT__: JSON.stringify(GIT_SHA),
		__APP_RIVET_NAMESPACE__: JSON.stringify(process.env.CF_PAGES_BRANCH),
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
	worker: {
		format: "es",
	},
});

async function shikiTransformer(): Promise<Plugin> {
	const cssVariableTheme = shiki.createCssVariablesTheme({
		name: "css-variables",
		variablePrefix: "--shiki-",
		variableDefaults: {},
		fontStyle: true,
	});

	let highlighter: shiki.Highlighter | undefined;

	return {
		name: "shiki",
		async transform(code, id) {
			if (id.includes("?shiki")) {
				highlighter ??= await shiki.getSingletonHighlighter({
					themes: [cssVariableTheme],
					langs: [
						"bash",
						"batch",
						"cpp",
						"csharp",
						"docker",
						"gdscript",
						"html",
						"ini",
						"js",
						"json",
						"json",
						"powershell",
						"ts",
						"typescript",
						"yaml",
						"http",
						"prisma",
					],
				});

				const params = new URLSearchParams(id.split("?")[1]);
				const output = highlighter.codeToHtml(code, {
					lang: params.get("lang") ?? "bash",
					theme: "css-variables",
					transformers: [transformerNotationFocus()],
				});
				return `export default ${JSON.stringify(
					output,
				)};export const source = ${JSON.stringify(code)}`;
			}
		},
	};
}
