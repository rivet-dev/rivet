#!/usr/bin/env node
/**
 * Build wrapper for rivetkit-napi.
 */
import { execFileSync } from "node:child_process";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const packageDir = dirname(dirname(fileURLToPath(import.meta.url)));
const require = createRequire(import.meta.url);

function resolveNapiCliBin() {
	try {
		const packageJsonPath = require.resolve("@napi-rs/cli/package.json", {
			paths: [packageDir],
		});
		const { bin } = require(packageJsonPath);
		return join(dirname(packageJsonPath), bin.napi);
	} catch (err) {
		throw new Error(
			"Missing @napi-rs/cli dependency. Run pnpm install before building @rivetkit/rivetkit-napi.",
			{ cause: err },
		);
	}
}

const args = process.argv.slice(2);
const extraFlags = args.includes("--release") ? ["--release"] : [];

// Explicit skip for environments that don't need the native binary (e.g.
// Docker engine-frontend build which only consumes TypeScript types).
if (process.env.SKIP_NAPI_BUILD === "1") {
	console.log(
		"[rivetkit-napi/build] SKIP_NAPI_BUILD=1 — skipping napi build",
	);
	process.exit(0);
}

// The per-actor inspector UI (frontend/dist/inspector-ui, embedded into
// rivetkit-core by its build.rs) must be built before this napi build runs.
// It is NOT built here: rivetkit-core's embed needs rivetkit/inspector-tab,
// which is downstream of this package in the build graph, so building it from
// the napi build would invert the dependency order. CI builds it via
// `turbo build:inspector-ui` in docker/build/*.Dockerfile before `napi build`;
// for local builds run `pnpm -F @rivetkit/engine-frontend build:inspector-ui`
// (or `turbo build:inspector-ui`) first.
const cmd = ["build", "--platform", ...extraFlags];
const napiCliBin = resolveNapiCliBin();

console.log(`[rivetkit-napi/build] running: napi ${cmd.join(" ")}`);
execFileSync(process.execPath, [napiCliBin, ...cmd], { stdio: "inherit" });
