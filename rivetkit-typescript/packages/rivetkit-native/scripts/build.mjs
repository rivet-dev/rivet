#!/usr/bin/env node
/**
 * Smart build wrapper for rivetkit-native.
 *
 * Skips the napi build if a prebuilt .node file already exists next to
 * this package (either a root-level `rivetkit-native.*.node` or one inside
 * a `npm/<platform>/` directory). This lets CI skip a redundant napi build
 * when the cross-compiled artifacts have already been downloaded from the
 * platform build jobs.
 *
 * Pass `--force` to always run the napi build.
 */
import { execSync } from "node:child_process";
import { existsSync, readdirSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const packageDir = join(__dirname, "..");

const args = process.argv.slice(2);
const force = args.includes("--force");
const releaseArg = args.find((a) => a === "--release");
const extraFlags = releaseArg ? ["--release"] : [];

// Explicit skip for environments that don't need the native binary (e.g.
// Docker engine-frontend build which only consumes TypeScript types).
if (!force && process.env.SKIP_NAPI_BUILD === "1") {
	console.log(
		"[rivetkit-native/build] SKIP_NAPI_BUILD=1 — skipping napi build",
	);
	process.exit(0);
}

function hasPrebuiltArtifact() {
	// Check for root-level .node files.
	const rootFiles = readdirSync(packageDir);
	if (rootFiles.some((f) => f.endsWith(".node"))) {
		return true;
	}
	// Check for any npm/<platform>/*.node files.
	const npmDir = join(packageDir, "npm");
	if (existsSync(npmDir) && statSync(npmDir).isDirectory()) {
		for (const entry of readdirSync(npmDir)) {
			const platDir = join(npmDir, entry);
			if (!statSync(platDir).isDirectory()) continue;
			const files = readdirSync(platDir);
			if (files.some((f) => f.endsWith(".node"))) {
				return true;
			}
		}
	}
	return false;
}

if (!force && hasPrebuiltArtifact()) {
	console.log(
		"[rivetkit-native/build] prebuilt .node artifact found — skipping napi build",
	);
	console.log("[rivetkit-native/build] use --force to rebuild from source");
	process.exit(0);
}

const cmd = ["napi", "build", "--platform", "--dts", "index.d.ts", ...extraFlags].join(" ");
console.log(`[rivetkit-native/build] running: ${cmd}`);
execSync(cmd, { stdio: "inherit", cwd: packageDir });
