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

// Build the per-actor inspector UI before compiling. rivetkit-core's build.rs
// embeds frontend/dist/inspector-ui at compile time (include_dir!); without it
// the napi ships an empty bundle and /inspector/ui/ returns
// inspector.ui_asset_not_found at runtime. Skip with SKIP_INSPECTOR_UI_BUILD=1
// for fast iteration when the frontend is already built.
if (process.env.SKIP_INSPECTOR_UI_BUILD !== "1") {
	console.log("[rivetkit-napi/build] building inspector UI frontend");
	execFileSync(
		"pnpm",
		["--filter", "@rivetkit/engine-frontend", "run", "build:inspector-ui"],
		{ stdio: "inherit" },
	);
}

const cmd = ["build", "--platform", ...extraFlags];
console.log(`[rivetkit-napi/build] running: napi ${cmd.join(" ")}`);
execFileSync("napi", cmd, { stdio: "inherit" });
