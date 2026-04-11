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
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";
import { discoverPackages } from "./discover-packages.js";

/** Walk up from this file to find the repo root (contains pnpm-workspace.yaml). */
function findRepoRoot(): string {
	let dir = dirname(fileURLToPath(import.meta.url));
	for (let i = 0; i < 10; i++) {
		if (existsSync(join(dir, "pnpm-workspace.yaml"))) return dir;
		dir = dirname(dir);
	}
	throw new Error("Could not locate repo root (no pnpm-workspace.yaml)");
}

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

const repoRoot = findRepoRoot();
process.chdir(repoRoot);
const packages = discoverPackages(repoRoot);
const packageNames = new Set(packages.map((p) => p.name));

/**
 * Meta packages that need `optionalDependencies` injected at publish time.
 * Each meta package's runtime loader requires the platform-specific package
 * for the current host — without the optionalDependencies, npm never installs
 * those and the require fails.
 *
 * The committed `package.json` files deliberately do NOT include these —
 * they'd pollute non-CI installs with broken version pins — so we inject
 * them here before publish.
 */
interface MetaPackageSpec {
	meta: string;
	platformPrefix: string;
}
const META_PACKAGES: MetaPackageSpec[] = [
	{
		meta: "@rivetkit/rivetkit-native",
		platformPrefix: "@rivetkit/rivetkit-native-",
	},
	{
		meta: "@rivetkit/engine-cli",
		platformPrefix: "@rivetkit/engine-cli-",
	},
];
const metaPlatformMap = new Map<string, string[]>(
	META_PACKAGES.map(({ meta, platformPrefix }) => [
		meta,
		packages
			.filter((p) => p.name.startsWith(platformPrefix))
			.map((p) => p.name)
			.sort(),
	]),
);

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

	// Inject optionalDependencies on meta packages so end users get the
	// right platform-specific binary via npm's os/cpu/libc resolution.
	const platformPkgs = metaPlatformMap.get(pkg.name);
	if (platformPkgs && platformPkgs.length > 0) {
		pkgJson.optionalDependencies = pkgJson.optionalDependencies ?? {};
		for (const platPkg of platformPkgs) {
			pkgJson.optionalDependencies[platPkg] = VERSION;
		}
	}

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
