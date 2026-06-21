#!/usr/bin/env node
// =============================================================================
// agent-os dependency manager  (rivet / r6  ->  agent-os)
// =============================================================================
//
// Single tool to control how this repo (rivet / r6) consumes agent-os, mirroring
// agent-os's own `scripts/secure-exec-dep.mjs` (which controls how agent-os
// consumes secure-exec). Same two-mode model:
//
//   pinned  (default for CI/release) — every agent-os dependency resolves from
//           its PUBLISHED artifact: npm `@rivet-dev/agent-os-*` from the registry
//           at the pinned version, and the Rust `agent-os-client` crate from a
//           vendored git rev. CI needs no sibling checkout.
//   local   (for hacking on agent-os) — every swappable dependency is redirected
//           at the sibling ../agent-os checkout: npm via `link:` and the cargo
//           `agent-os-client` crate via `path = ".../agent-os/crates/client"`.
//           This is the local dev loop: edit agent-os, rebuild here, no publish.
//
// Commands:
//   node scripts/agent-os-dep.mjs local
//   node scripts/agent-os-dep.mjs pinned
//   node scripts/agent-os-dep.mjs set-version <version>   # bump pinned npm version
//   node scripts/agent-os-dep.mjs status
//
// After `local`/`pinned`/`set-version`, run `pnpm install` and a cargo build so
// the lockfiles pick up the new resolution.
//
// See the rivet CLAUDE.md "Agent OS dependency (local dev vs preview publish)"
// section for the full workflow.
// =============================================================================

import { readFileSync, writeFileSync, existsSync, readdirSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const AGENT_OS_REL = "../agent-os"; // sibling checkout, same convention as agent-os -> ../secure-exec
const AGENT_OS_ABS = path.resolve(ROOT, AGENT_OS_REL);

// npm `@rivet-dev/agent-os-*` package -> its source dir under the agent-os repo.
// (Published name -> repo subpath. `common` is renamed from registry/software at
// publish time; we link its source dir when present.)
const NPM_PKGS = {
	"@rivet-dev/agent-os-core": "packages/core",
	"@rivet-dev/agent-os-sidecar": "packages/sidecar-binary",
	"@rivet-dev/agent-os-sandbox": "packages/agent-os-sandbox",
	"@rivet-dev/agent-os-pi": "registry/agent/pi",
	"@rivet-dev/agent-os-common": "registry/software/common",
};

// Rust crate -> its source dir under the agent-os repo.
const CRATES = {
	"agent-os-client": "crates/client",
};

// Pinned (published) versions, used by `pinned` mode. `set-version` rewrites
// these in place. agent-os publishes core/sidecar/pi/sandbox on one cadence and
// the renamed software packages (common) on another, hence two seeds.
const SEED_VERSION = "0.0.0-main.8794200";
const SEED_SOFTWARE_VERSION = "0.0.260331072558";
// Pinned git rev for the Rust crate (no crates.io publish). Empty => `pinned`
// leaves the cargo dep as-is and warns; set it once agent-os has a tagged rev.
const PINNED_GIT = { repo: "https://github.com/rivet-dev/agent-os.git", rev: "" };

const STATE_FILE = path.join(ROOT, "scripts", ".agent-os-dep.json");

// ---------------------------------------------------------------------------
const escapeRe = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

function loadState() {
	if (existsSync(STATE_FILE)) {
		try {
			return JSON.parse(readFileSync(STATE_FILE, "utf8"));
		} catch {
			/* fall through to seed */
		}
	}
	return { version: SEED_VERSION, softwareVersion: SEED_SOFTWARE_VERSION };
}
function saveState(state) {
	writeFileSync(STATE_FILE, `${JSON.stringify(state, null, 2)}\n`);
}
function pinnedVersionFor(name, state) {
	return name === "@rivet-dev/agent-os-common"
		? state.softwareVersion
		: state.version;
}

// ---------------------------------------------------------------------------
// consumer discovery
// ---------------------------------------------------------------------------
// Every package.json under these groups is a potential npm consumer; every
// Cargo.toml under the workspace is a potential cargo consumer.
function packageManifests() {
	const out = [];
	const groups = ["examples", "rivetkit-typescript/packages"];
	for (const group of groups) {
		const base = path.join(ROOT, group);
		if (!existsSync(base)) continue;
		for (const entry of readdirSync(base, { withFileTypes: true })) {
			if (!entry.isDirectory()) continue;
			const p = path.join(base, entry.name, "package.json");
			if (existsSync(p)) out.push(p);
		}
	}
	return out;
}
function cargoManifests() {
	// Only the two crates that path-depend on agent-os today; discovered by grep
	// to stay correct if more are added.
	const out = [];
	const candidates = [
		"rivetkit-rust/packages/rivetkit-agent-os/Cargo.toml",
		"rivetkit-typescript/packages/rivetkit-napi/Cargo.toml",
	];
	for (const rel of candidates) {
		const p = path.join(ROOT, rel);
		if (existsSync(p)) out.push(p);
	}
	return out;
}

// Relative path from a manifest's dir to an agent-os subpath.
function relTo(manifestPath, subpath) {
	const target = path.join(AGENT_OS_ABS, subpath);
	let rel = path.relative(path.dirname(manifestPath), target);
	if (!rel.startsWith(".")) rel = `./${rel}`;
	return rel;
}

// ---------------------------------------------------------------------------
// npm: rewrite consumer dep values
// ---------------------------------------------------------------------------
function rewriteNpm(mode, state) {
	let changed = 0;
	for (const manifest of packageManifests()) {
		let text = readFileSync(manifest, "utf8");
		const before = text;
		for (const [name, subpath] of Object.entries(NPM_PKGS)) {
			if (!new RegExp(`"${escapeRe(name)}"\\s*:`).test(text)) continue;
			let value;
			if (mode === "local") {
				const target = path.join(AGENT_OS_ABS, subpath);
				if (!existsSync(target)) continue; // skip packages with no local source
				value = `link:${relTo(manifest, subpath)}`;
			} else {
				value = pinnedVersionFor(name, state);
			}
			const re = new RegExp(`("${escapeRe(name)}"\\s*:\\s*)"[^"]*"`, "g");
			text = text.replace(re, `$1"${value}"`);
		}
		if (text !== before) {
			writeFileSync(manifest, text);
			changed++;
		}
	}
	return changed;
}

// ---------------------------------------------------------------------------
// cargo: rewrite agent-os-client path/git dep
// ---------------------------------------------------------------------------
function rewriteCargo(mode) {
	let changed = 0;
	for (const manifest of cargoManifests()) {
		const lines = readFileSync(manifest, "utf8").split("\n");
		let touched = false;
		const out = lines.map((line) => {
			const m = line.match(/^(\s*)([A-Za-z0-9_-]+)\s*=\s*\{(.*)\}\s*$/);
			if (!m) return line;
			const [, indent, key, body] = m;
			const pkg = (body.match(/package\s*=\s*"([^"]+)"/) || [])[1] || key;
			const subpath = CRATES[pkg];
			if (!subpath) return line;
			touched = true;
			const parts = [];
			if (body.includes("package =")) parts.push(`package = "${pkg}"`);
			if (mode === "local") {
				parts.push(`path = "${relTo(manifest, subpath)}"`);
			} else if (PINNED_GIT.rev) {
				parts.push(`git = "${PINNED_GIT.repo}"`, `rev = "${PINNED_GIT.rev}"`);
			} else {
				// No published crate + no pinned rev: leave the line unchanged and warn.
				console.warn(
					`  [warn] ${path.relative(ROOT, manifest)}: '${pkg}' has no pinned git rev; left unchanged. Set PINNED_GIT.rev to enable pinned cargo mode.`,
				);
				return line;
			}
			return `${indent}${key} = { ${parts.join(", ")} }`;
		});
		if (touched) {
			writeFileSync(manifest, out.join("\n"));
			changed++;
		}
	}
	return changed;
}

// ---------------------------------------------------------------------------
// status detection
// ---------------------------------------------------------------------------
function npmMode() {
	for (const manifest of packageManifests()) {
		const text = readFileSync(manifest, "utf8");
		for (const name of Object.keys(NPM_PKGS)) {
			if (new RegExp(`"${escapeRe(name)}"\\s*:\\s*"link:`).test(text)) return "local";
		}
	}
	return "pinned";
}
function cargoMode() {
	for (const manifest of cargoManifests()) {
		const text = readFileSync(manifest, "utf8");
		if (/agent-os-client\s*=\s*\{[^}]*\bpath\s*=/.test(text)) return "local";
	}
	return "pinned";
}
function currentMode() {
	const n = npmMode();
	const c = cargoMode();
	return n === c ? n : `hybrid(npm=${n},cargo=${c})`;
}

// ---------------------------------------------------------------------------
const [cmd, arg] = process.argv.slice(2);
switch (cmd) {
	case "local": {
		const n = rewriteNpm("local", loadState());
		const c = rewriteCargo("local");
		console.log(`agent-os deps -> LOCAL (../agent-os via link:/path). npm:${n} cargo:${c} manifests.`);
		console.log("Run: pnpm install   (and a cargo build) to refresh lockfiles.");
		break;
	}
	case "pinned": {
		const n = rewriteNpm("pinned", loadState());
		const c = rewriteCargo("pinned");
		console.log(`agent-os deps -> PINNED (published versions). npm:${n} cargo:${c} manifests.`);
		console.log("Run: pnpm install to refresh the lockfile.");
		break;
	}
	case "set-version": {
		if (!arg) {
			console.error("usage: set-version <version>");
			process.exit(1);
		}
		const state = loadState();
		state.version = arg;
		saveState(state);
		if (npmMode() === "pinned") rewriteNpm("pinned", state);
		console.log(`pinned @rivet-dev/agent-os-* version set to ${arg}.`);
		console.log("Run: pnpm install to refresh the lockfile.");
		break;
	}
	case "status": {
		const state = loadState();
		console.log(`mode: ${currentMode()}`);
		console.log(`sibling: ${AGENT_OS_ABS} (${existsSync(AGENT_OS_ABS) ? "present" : "MISSING"})`);
		console.log(`pinned npm version: ${state.version} (software: ${state.softwareVersion})`);
		console.log(`pinned cargo rev: ${PINNED_GIT.rev || "(none — cargo stays local until set)"}`);
		break;
	}
	default:
		console.error("usage: agent-os-dep.mjs <local|pinned|set-version <v>|status>");
		process.exit(1);
}
