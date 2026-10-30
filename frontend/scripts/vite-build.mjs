#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";

const require = createRequire(import.meta.url);

const BUILD_HEAP_LIMIT_MB = 8192;
const ENGINE_BUILD_DEFAULTS = {
	BASE_URL: "/ui/",
	VITE_APP_API_URL: "__SAME__",
	VITE_FEATURE_FLAGS: "",
};

const args = process.argv.slice(2);
const isEngineBuild = args.includes("--engine");
const viteArgs = args.filter((arg) => arg !== "--engine");

function resolveViteBin() {
	const packageJsonPath = require.resolve("vite/package.json");
	const { bin } = require(packageJsonPath);
	return join(dirname(packageJsonPath), bin.vite);
}

function engineBuildEnv() {
	return {
		...process.env,
		BASE_URL: ENGINE_BUILD_DEFAULTS.BASE_URL,
		VITE_APP_API_URL:
			process.env.VITE_APP_API_URL ?? ENGINE_BUILD_DEFAULTS.VITE_APP_API_URL,
		VITE_FEATURE_FLAGS:
			process.env.VITE_FEATURE_FLAGS ?? ENGINE_BUILD_DEFAULTS.VITE_FEATURE_FLAGS,
	};
}

execFileSync(
	process.execPath,
	[
		`--max-old-space-size=${BUILD_HEAP_LIMIT_MB}`,
		resolveViteBin(),
		"build",
		"--mode=production",
		...viteArgs,
	],
	{
		stdio: "inherit",
		env: isEngineBuild ? engineBuildEnv() : process.env,
	},
);
