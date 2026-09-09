#!/usr/bin/env node
/**
 * Stage the built Inspector UI bundle into the rivetkit-core crate so it ships
 * to crates.io.
 *
 * `frontend/dist/inspector-{ui,tab}` lives outside the crate and is absent from
 * the `.crate` archive, so a published crate would embed the empty fallback and
 * serve `inspector.ui_asset_not_found`. This copies the built assets into the
 * in-crate `inspector-dist/` directory, which `build.rs` embeds via
 * `include_dir!`.
 *
 * Run the frontend build first:
 *   pnpm turbo build:inspector-ui -F @rivetkit/engine-frontend
 * then:
 *   node scripts/stage-inspector-bundle.mjs
 *
 * Source maps are stripped: they are debug-only, bloat the crate, and the crate
 * has no need to serve `*.map` requests.
 */
import { cpSync, existsSync, mkdirSync, rmSync, statSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const crateDir = dirname(__dirname);
const repoRoot = resolve(crateDir, "../../..");
const frontendDist = join(repoRoot, "frontend", "dist");
const stageRoot = join(crateDir, "inspector-dist");

// Each bundle plus the marker file that proves it was actually built.
const BUNDLES = [
	{ name: "inspector-ui", marker: "index.html" },
	{ name: "inspector-tab", marker: "styles.css" },
];

function fail(message) {
	console.error(`stage-inspector-bundle: ${message}`);
	process.exit(1);
}

let strippedMaps = 0;
let strippedBytes = 0;

for (const { name, marker } of BUNDLES) {
	const source = join(frontendDist, name);
	const markerPath = join(source, marker);
	if (!existsSync(markerPath)) {
		fail(
			`missing ${name}/${marker} at ${markerPath}. Build it first:\n` +
				"  pnpm turbo build:inspector-ui -F @rivetkit/engine-frontend",
		);
	}

	const dest = join(stageRoot, name);
	rmSync(dest, { recursive: true, force: true });
	mkdirSync(dest, { recursive: true });

	cpSync(source, dest, {
		recursive: true,
		filter: (src) => {
			if (src.endsWith(".map")) {
				try {
					strippedBytes += statSync(src).size;
				} catch {}
				strippedMaps += 1;
				return false;
			}
			return true;
		},
	});

	console.log(`staged ${name} -> ${dest}`);
}

if (strippedMaps > 0) {
	console.log(
		`stripped ${strippedMaps} source map file(s) (${Math.round(strippedBytes / 1024)} KiB) from the published crate bundle`,
	);
}
