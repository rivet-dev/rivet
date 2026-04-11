/**
 * Discover all packages to be published as part of a preview-publish run.
 *
 * Returns a stable, topologically ordered list:
 *   1. rivetkit-native platform packages (must come first since they are
 *      `optionalDependencies` of the `@rivetkit/rivetkit-native` meta package)
 *   2. pnpm workspace packages (rivetkit, @rivetkit/*)
 *   3. engine/sdks/typescript/* packages
 *   4. shared/typescript/* packages
 */
import { execSync } from "node:child_process";
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative, resolve } from "node:path";

export interface Package {
	name: string;
	/** Directory containing the package.json (absolute). */
	dir: string;
	/** Directory relative to repo root. */
	relDir: string;
}

/**
 * Packages to exclude from publishing. Must stay in sync with the release
 * workflow filters.
 */
const SKIP = new Set([
	"@rivetkit/shared-data",
	"@rivetkit/engine-frontend",
	"@rivetkit/mcp-hub",
]);

function isPublishable(pkgJson: {
	name?: string;
	private?: boolean;
}): boolean {
	if (!pkgJson.name) return false;
	if (pkgJson.private) return false;
	if (SKIP.has(pkgJson.name)) return false;
	return true;
}

function readPackageJson(
	dir: string,
): { name?: string; private?: boolean } | null {
	const pkgPath = join(dir, "package.json");
	if (!existsSync(pkgPath)) return null;
	try {
		return JSON.parse(readFileSync(pkgPath, "utf8"));
	} catch {
		return null;
	}
}

export function discoverPackages(repoRoot: string): Package[] {
	const packages: Package[] = [];
	const seen = new Set<string>();

	const add = (dir: string) => {
		const absDir = resolve(dir);
		const pkg = readPackageJson(absDir);
		if (!pkg) return;
		if (!pkg.name) return;
		if (!isPublishable(pkg)) return;
		if (seen.has(pkg.name)) return;
		seen.add(pkg.name);
		packages.push({
			name: pkg.name,
			dir: absDir,
			relDir: relative(repoRoot, absDir),
		});
	};

	// 1. Platform-specific packages first. These are optionalDependencies of
	//    their meta packages and must exist on npm before the meta package
	//    resolves at install time.
	//    - rivetkit-native: the N-API addon (.node files)
	//    - engine-cli: the rivet-engine binary
	for (const metaRelDir of [
		"rivetkit-typescript/packages/rivetkit-native/npm",
		"rivetkit-typescript/packages/engine-cli/npm",
	]) {
		const npmDir = join(repoRoot, metaRelDir);
		if (!existsSync(npmDir)) continue;
		for (const entry of readdirSync(npmDir).sort()) {
			const platDir = join(npmDir, entry);
			if (!statSync(platDir).isDirectory()) continue;
			add(platDir);
		}
	}

	// 2. pnpm workspace packages (rivetkit + @rivetkit/*).
	const pnpmList = execSync("pnpm -r list --json", {
		cwd: repoRoot,
		encoding: "utf8",
		maxBuffer: 16 * 1024 * 1024,
	});
	const workspacePkgs: Array<{
		name: string;
		path: string;
		private?: boolean;
	}> = JSON.parse(pnpmList);
	for (const p of workspacePkgs) {
		if (!p.name) continue;
		if (!p.name.startsWith("@rivetkit/") && p.name !== "rivetkit") continue;
		add(p.path);
	}

	// 3. Engine SDK TypeScript packages.
	const engineSdkDir = join(repoRoot, "engine/sdks/typescript");
	if (existsSync(engineSdkDir)) {
		for (const entry of readdirSync(engineSdkDir).sort()) {
			add(join(engineSdkDir, entry));
		}
	}

	// 4. Shared TypeScript packages.
	const sharedDir = join(repoRoot, "shared/typescript");
	if (existsSync(sharedDir)) {
		for (const entry of readdirSync(sharedDir).sort()) {
			add(join(sharedDir, entry));
		}
	}

	return packages;
}
