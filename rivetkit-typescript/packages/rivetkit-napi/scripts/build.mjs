#!/usr/bin/env node
/**
 * Build wrapper for rivetkit-napi.
 */
import { execFileSync } from "node:child_process";

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

const cmd = ["build", "--platform", ...extraFlags];
console.log(`[rivetkit-napi/build] running: napi ${cmd.join(" ")}`);
execFileSync("napi", cmd, { stdio: "inherit" });
