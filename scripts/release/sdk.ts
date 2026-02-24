import { $ } from "execa";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import type { ReleaseOpts } from "./main";

// Packages to exclude from build and publish.
// These are either private packages or have dependencies that aren't available in CI.
// This should match the filters in .github/workflows/pkg-pr-new.yaml
export const EXCLUDED_RIVETKIT_PACKAGES = [
	"@rivetkit/shared-data",
	"@rivetkit/engine-frontend",
	"@rivetkit/mcp-hub",
] as const;

async function npmVersionExists(
	packageName: string,
	version: string,
): Promise<boolean> {
	console.log(
		`==> Checking if NPM version exists: ${packageName}@${version}`,
	);
	try {
		await $({
			stdout: "ignore",
			stderr: "pipe",
		})`npm view ${packageName}@${version} version`;
		return true;
	} catch (error: any) {
		if (error.stderr) {
			if (
				!error.stderr.includes(
					`No match found for version ${version}`,
				) &&
				!error.stderr.includes(
					`'${packageName}@${version}' is not in this registry.`,
				)
			) {
				throw new Error(
					`unexpected npm view version output: ${error.stderr}`,
				);
			}
		}
		return false;
	}
}

async function getRivetkitPackages(opts: ReleaseOpts): Promise<string[]> {
	const { stdout } = await $({
		cwd: opts.root,
	})`pnpm -r list --json`;
	const allPackages = JSON.parse(stdout.trim());

	return allPackages
		.filter(
			(pkg: any) =>
				(pkg.name === "rivetkit" || pkg.name.startsWith("@rivetkit/")) &&
				// Exclude engine packages as they're handled separately in enginePackagePaths
				!pkg.name.startsWith("@rivetkit/engine-") &&
				// Exclude packages that shouldn't be published
				!EXCLUDED_RIVETKIT_PACKAGES.includes(pkg.name),
		)
		.map((pkg: any) => pkg.name);
}

export async function publishSdk(opts: ReleaseOpts) {
	// Build rivetkit packages first
	console.log("==> Building rivetkit packages");

	// Build exclusion filters for packages that shouldn't be built
	const excludeFilters = EXCLUDED_RIVETKIT_PACKAGES.flatMap((pkg) => [
		"-F",
		`!${pkg}`,
	]);

	try {
		await $({
			stdio: "inherit",
			cwd: opts.root,
		})`pnpm build -F rivetkit -F @rivetkit/* ${excludeFilters}`;

		// Pack the inspector UI tarball into rivetkit's dist.
		// This is separate from `build` because build:pack-inspector depends on
		// @rivetkit/engine-frontend#build:inspector which is in the exclusion
		// list. Turbo resolves dependsOn transitively regardless of filters.
		await $({
			stdio: "inherit",
			cwd: opts.root,
		})`npx turbo build:pack-inspector -F rivetkit`;

		console.log("✅ Rivetkit packages built");
	} catch (err) {
		console.error("❌ Failed to build rivetkit packages");
		throw err;
	}

	// Get list of packages to publish
	const enginePackagePaths = [
		`${opts.root}/engine/sdks/typescript/runner`,
		`${opts.root}/engine/sdks/typescript/runner-protocol`,
		`${opts.root}/engine/sdks/typescript/api-full`,
	];

	const rivetkitPackages = await getRivetkitPackages(opts);

	// Publish engine SDKs
	for (const path of enginePackagePaths) {
		// Read package.json to get the name
		const packageJsonPath = join(path, "package.json");
		const packageJson = JSON.parse(
			await readFile(packageJsonPath, "utf-8"),
		);
		const name = packageJson.name;

		// Check if version already exists
		let versionExists = false;
		versionExists = await npmVersionExists(name, opts.version);

		if (versionExists) {
			console.log(
				`Version ${opts.version} of ${name} already exists. Skipping...`,
			);
			continue;
		}

		// Publish
		console.log(`==> Publishing to NPM: ${name}@${opts.version}`);

		// Add --tag flag for release candidates
		const isReleaseCandidate = opts.version.includes("-rc.");
		const tag = isReleaseCandidate ? "rc" : "latest";

		await $({
			stdio: "inherit",
		})`pnpm --filter ${name} publish --access public --tag ${tag} --no-git-checks`;
	}

	// Publish rivetkit packages
	for (const name of rivetkitPackages) {
		// Check if version already exists
		let versionExists = false;
		versionExists = await npmVersionExists(name, opts.version);

		if (versionExists) {
			console.log(
				`Version ${opts.version} of ${name} already exists. Skipping...`,
			);
			continue;
		}

		// Publish
		console.log(`==> Publishing to NPM: ${name}@${opts.version}`);

		// Add --tag flag for release candidates
		const isReleaseCandidate = opts.version.includes("-rc.");
		const tag = isReleaseCandidate ? "rc" : "latest";

		await $({
			stdio: "inherit",
			cwd: opts.root,
		})`pnpm --filter ${name} publish --access public --tag ${tag} --no-git-checks`;
	}
}
