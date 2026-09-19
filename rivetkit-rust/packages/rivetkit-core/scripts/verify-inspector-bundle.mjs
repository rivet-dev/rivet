#!/usr/bin/env node
/**
 * Release check: prove the Inspector UI actually ships in the rivetkit-core
 * `.crate` and is served, rather than degrading to
 * `inspector.ui_asset_not_found`.
 *
 * Two independent assertions:
 *   1. Archive inclusion: assert the Inspector UI index.html and the tab
 *      stylesheet are in the exact file list cargo would pack (`cargo package
 *      --list`). This catches the root-cause bug (asset missing from the
 *      published artifact) that the empty-bundle fallback otherwise hides.
 *
 *      `--list` is used instead of building the `.crate`: full packaging strips
 *      path deps and resolves the exact-pinned sibling crates against
 *      crates.io, which are not published yet at this point in the ordered
 *      publish run. `--list` uses the workspace path deps, so it never touches
 *      the registry while reporting the same file set.
 *   2. Runtime serving: build the crate and assert `GET /inspector/ui/` returns
 *      index.html, not the `ui_asset_not_found` JSON error, via the
 *      `inspector_bundle` integration test (gated on
 *      RIVETKIT_ASSERT_INSPECTOR_BUNDLE so ordinary `cargo test` runs without a
 *      built frontend still pass).
 *
 * Run `node scripts/stage-inspector-bundle.mjs` first.
 */
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, statSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const crateDir = dirname(__dirname);
const repoRoot = resolve(crateDir, "../../..");

function fail(message) {
	console.error(`verify-inspector-bundle: ${message}`);
	process.exit(1);
}

function assertHtmlIndex(path, label) {
	if (!existsSync(path)) {
		fail(
			`${label} missing (${path}). The published crate would serve inspector.ui_asset_not_found.`,
		);
	}
	const html = readFileSync(path, "utf-8").toLowerCase();
	if (!html.includes("<!doctype html") && !html.includes("<html")) {
		fail(`${label} is not HTML (${path}).`);
	}
	if (statSync(path).size < 200) {
		fail(`${label} is suspiciously small (${path}); looks like a placeholder.`);
	}
}

// --- Pre-check: staged in-crate assets exist -------------------------------

const stagedIndex = join(crateDir, "inspector-dist", "inspector-ui", "index.html");
const stagedTabCss = join(crateDir, "inspector-dist", "inspector-tab", "styles.css");
assertHtmlIndex(stagedIndex, "staged inspector-ui/index.html");
if (!existsSync(stagedTabCss)) {
	fail(`staged inspector-tab/styles.css missing (${stagedTabCss}).`);
}

// --- 1. Archive inclusion --------------------------------------------------

console.log("listing packaged files for rivetkit-core...");
const listed = execFileSync(
	"cargo",
	["package", "-p", "rivetkit-core", "--allow-dirty", "--no-verify", "--list"],
	{ cwd: repoRoot, encoding: "utf-8" },
);
const files = new Set(listed.split("\n").map((l) => l.trim()));

for (const required of [
	"inspector-dist/inspector-ui/index.html",
	"inspector-dist/inspector-tab/styles.css",
]) {
	if (!files.has(required)) {
		fail(
			`${required} is not in the rivetkit-core package file list. ` +
				"The published crate would serve inspector.ui_asset_not_found.",
		);
	}
}
console.log("ok: rivetkit-core packages inspector-ui/index.html and inspector-tab/styles.css");

// --- 2. Runtime serving ----------------------------------------------------

console.log("asserting GET /inspector/ui/ serves index.html...");
execFileSync(
	"cargo",
	["test", "-p", "rivetkit-core", "--test", "inspector_bundle"],
	{
		cwd: repoRoot,
		stdio: "inherit",
		env: { ...process.env, RIVETKIT_ASSERT_INSPECTOR_BUNDLE: "1" },
	},
);

console.log("verify-inspector-bundle: ok");
