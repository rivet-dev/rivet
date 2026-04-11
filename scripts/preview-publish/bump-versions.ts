#!/usr/bin/env tsx
/**
 * Rewrite all publishable package versions to the preview-publish pre-release
 * version and rewrite every `workspace:*` (and `workspace:^`, `workspace:~`)
 * dependency reference to the same literal version.
 *
 * Usage:
 *   tsx scripts/preview-publish/bump-versions.ts --version 2.2.1-pr.4600.abc1234
 *   tsx scripts/preview-publish/bump-versions.ts --version 2.2.1-pr.4600.abc1234 --dry-run
 */
import { readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { parseArgs } from "node:util";
import { discoverPackages } from "./discover-packages.js";

const { values } = parseArgs({
	options: {
		version: { type: "string" },
		"dry-run": { type: "boolean", default: false },
	},
});

if (!values.version) {
	console.error("--version is required");
	process.exit(1);
}
const VERSION = values.version;
const DRY_RUN = values["dry-run"] ?? false;

const repoRoot = resolve(process.cwd());
const packages = discoverPackages(repoRoot);
const packageNames = new Set(packages.map((p) => p.name));

interface PackageJson {
	name?: string;
	version?: string;
	dependencies?: Record<string, string>;
	devDependencies?: Record<string, string>;
	peerDependencies?: Record<string, string>;
	optionalDependencies?: Record<string, string>;
}

const DEP_FIELDS = [
	"dependencies",
	"devDependencies",
	"peerDependencies",
	"optionalDependencies",
] as const;

let updated = 0;
for (const pkg of packages) {
	const pkgJsonPath = join(pkg.dir, "package.json");
	const raw = readFileSync(pkgJsonPath, "utf8");
	const pkgJson: PackageJson = JSON.parse(raw);

	pkgJson.version = VERSION;

	for (const field of DEP_FIELDS) {
		const deps = pkgJson[field];
		if (!deps) continue;
		for (const [dep, spec] of Object.entries(deps)) {
			const isWorkspace =
				typeof spec === "string" && spec.startsWith("workspace:");
			if (!isWorkspace) continue;
			// Only rewrite deps that are in our published set so we don't
			// accidentally point an internal-only dep at a non-existent version.
			const isOurPkg =
				packageNames.has(dep) ||
				dep.startsWith("@rivetkit/") ||
				dep === "rivetkit";
			if (!isOurPkg) continue;
			deps[dep] = VERSION;
		}
	}

	// Tab-indented, trailing newline — matches the repo convention.
	const newContent = `${JSON.stringify(pkgJson, null, "\t")}\n`;
	if (DRY_RUN) {
		console.log(`[dry-run] would update ${pkg.name} -> ${VERSION}`);
	} else {
		writeFileSync(pkgJsonPath, newContent);
		console.log(`updated ${pkg.name} -> ${VERSION}`);
	}
	updated++;
}

console.log(`\nTotal: ${updated} package.json files updated to ${VERSION}`);
