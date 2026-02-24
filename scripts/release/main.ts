#!/usr/bin/env tsx

import * as fs from "node:fs/promises";
import * as path from "node:path";
import * as url from "node:url";
import { $ } from "execa";
import { program } from "commander";
import * as semver from "semver";
import { buildJsArtifacts } from "./build-artifacts";
import { promoteArtifacts } from "./promote-artifacts";
import { tagDocker } from "./docker";
import {
	createAndPushTag,
	createGitHubRelease,
	validateGit,
} from "./git";
import { EXCLUDED_RIVETKIT_PACKAGES, publishSdk } from "./sdk";
import { updateVersion } from "./update_version";
import { assert, assertEquals, assertExists, fetchGitRef, listReleasesObjects, versionOrCommitToRef } from "./utils";

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
const ROOT_DIR = path.resolve(__dirname, "..", "..");

export interface ReleaseOpts {
	root: string;
	version: string;
	latest: boolean;
	/** Commit to publish release for. */
	commit: string;
	/** Optional version to reuse artifacts and Docker images from instead of building. */
	reuseEngineVersion?: string;
}

async function getAllGitVersions(): Promise<string[]> {
	try {
		// Fetch tags to ensure we have the latest
		// Use --force to overwrite local tags that conflict with remote
		try {
			await $`git fetch --tags --force --quiet`;
		} catch (fetchError) {
			console.warn("Warning: Could not fetch remote tags, using local tags only");
		}

		// Get all version tags
		const result = await $`git tag -l v*`;
		const tags = result.stdout.trim().split("\n").filter(Boolean);

		if (tags.length === 0) {
			return [];
		}

		// Parse and sort all versions (newest first)
		const versions = tags
			.map(tag => tag.replace(/^v/, ""))
			.filter(v => semver.valid(v))
			.sort((a, b) => semver.rcompare(a, b));

		return versions;
	} catch (error) {
		console.warn("Warning: Could not get git tags:", error);
		return [];
	}
}

async function getLatestGitVersion(): Promise<string | null> {
	const versions = await getAllGitVersions();

	if (versions.length === 0) {
		return null;
	}

	// Find the latest version (excluding prereleases)
	const stableVersions = versions.filter(v => {
		const parsed = semver.parse(v);
		return parsed && parsed.prerelease.length === 0;
	});

	return stableVersions[0] || null;
}

async function shouldTagAsLatest(newVersion: string): Promise<boolean> {
	// Check if version has prerelease identifier (e.g., 1.0.0-rc.1)
	const parsedVersion = semver.parse(newVersion);
	if (!parsedVersion) {
		throw new Error(`Invalid semantic version: ${newVersion}`);
	}

	// If it has a prerelease identifier, it's not latest
	if (parsedVersion.prerelease.length > 0) {
		return false;
	}

	// Get the latest version from git tags
	const latestGitVersion = await getLatestGitVersion();

	// If no previous versions exist, this is the latest
	if (!latestGitVersion) {
		return true;
	}

	// Check if new version is greater than the latest git version
	return semver.gt(newVersion, latestGitVersion);
}

async function validateReuseVersion(version: string): Promise<void> {
	console.log(`Validating that ${version} exists...`);

	const isVersion = version.includes(".");

	if (isVersion) {
		// When reusing a version, check version-tagged Docker manifests and
		// version-prefixed S3 paths. This works even when the referenced
		// version itself reused engine from an earlier release.
		console.log(`Checking version-tagged Docker manifests for ${version}...`);
		try {
			await $({ stdio: "inherit" })`docker manifest inspect rivetkit/engine:slim-${version}`;
			console.log("✅ Docker manifest exists");
		} catch (error) {
			throw new Error(
				`Docker manifest rivetkit/engine:slim-${version} does not exist. Error: ${error}`,
			);
		}

		console.log(`Checking S3 artifacts for version ${version}...`);
		const s3Prefix = `rivet/${version}/`;
		const files = await listReleasesObjects(s3Prefix);

		if (!Array.isArray(files?.Contents) || files.Contents.length === 0) {
			throw new Error(
				`No S3 artifacts found for version ${version} under ${s3Prefix}`,
			);
		}

		console.log(`✅ S3 artifacts exist (${files.Contents.length} files found)`);
	} else {
		// When reusing a raw commit, check per-arch Docker images and
		// commit-prefixed S3 paths.
		const ref = versionOrCommitToRef(version);
		await fetchGitRef(ref);

		let shortCommit: string;
		try {
			const result = await $`git rev-parse ${ref}`;
			const fullCommit = result.stdout.trim();
			shortCommit = fullCommit.slice(0, 7);
			console.log(`✅ Found ${ref} (commit ${shortCommit})`);
		} catch (error) {
			throw new Error(
				`${version} does not exist in git. Make sure ${ref} exists in the repository.`,
			);
		}

		console.log(`Checking Docker images for ${shortCommit}...`);
		try {
			await $({ stdio: "inherit" })`docker manifest inspect rivetkit/engine:slim-${shortCommit}-amd64`;
			await $({ stdio: "inherit" })`docker manifest inspect rivetkit/engine:slim-${shortCommit}-arm64`;
			console.log("✅ Docker images exist");
		} catch (error) {
			throw new Error(
				`Docker images for commit ${shortCommit} do not exist. Error: ${error}`,
			);
		}

		console.log(`Checking S3 artifacts for ${shortCommit}...`);
		const s3Prefix = `rivet/${shortCommit}/`;
		const files = await listReleasesObjects(s3Prefix);

		if (!Array.isArray(files?.Contents) || files.Contents.length === 0) {
			throw new Error(
				`No S3 artifacts found for commit ${shortCommit} under ${s3Prefix}`,
			);
		}

		console.log(`✅ S3 artifacts exist (${files.Contents.length} files found)`);
	}
}

interface BuildAndCheckOpts {
	ci: boolean;
}

async function runBuildAndChecks(opts: ReleaseOpts, { ci }: BuildAndCheckOpts) {
	const ciLabel = ci ? "CI" : "Local";
	console.log(`Running ${ciLabel} build and checks...`);

	// Build exclusion filters for CI (excludes website and packages that shouldn't be built in CI)
	const excludeFilters = ci
		? [
				"-F",
				"!rivet-website",
				...EXCLUDED_RIVETKIT_PACKAGES.flatMap((pkg) => ["-F", `!${pkg}`]),
			]
		: [];

	// Type check
	console.log("Checking types...");
	try {
		// Build rivetkit packages first since some examples depend on them
		console.log("Building rivetkit packages...");
		await $({
			stdio: "inherit",
			cwd: opts.root,
		})`pnpm build --force -F rivetkit -F @rivetkit/* ${excludeFilters}`;
		console.log("✅ Rivetkit packages built");

		// --force to skip cache in case of Turborepo bugs
		await $({ stdio: "inherit", cwd: opts.root })`pnpm check-types --force ${excludeFilters}`;
		console.log("✅ Type check passed");
	} catch (err) {
		console.error("❌ Type check failed");
		throw err;
	}

	// Cargo check
	if (!ci) {
		console.log("Running cargo check...");
		try {
			await $({ stdio: "inherit", cwd: opts.root })`cargo check`;
			console.log("✅ Cargo check passed");
		} catch (err) {
			console.error("❌ Cargo check failed");
			throw err;
		}
	}

	console.log(`✅ All ${ciLabel} checks passed`);
}

async function getVersionFromArgs(opts: {
	version?: string;
	major?: boolean;
	minor?: boolean;
	patch?: boolean;
}): Promise<string> {
	// Check if explicit version is provided via --version flag
	if (opts.version) {
		return opts.version;
	}

	// Check for version bump flags
	if (!opts.major && !opts.minor && !opts.patch) {
		throw new Error(
			"Must provide either --version, --major, --minor, or --patch",
		);
	}

	// Get latest version from git tags and calculate new one
	const latestVersion = await getLatestGitVersion();
	if (!latestVersion) {
		throw new Error(
			"No existing version tags found. Use --version to set an explicit version.",
		);
	}
	console.log(`Latest git version: ${latestVersion}`);

	let newVersion: string | null = null;

	if (opts.major) {
		newVersion = semver.inc(latestVersion, "major");
	} else if (opts.minor) {
		newVersion = semver.inc(latestVersion, "minor");
	} else if (opts.patch) {
		newVersion = semver.inc(latestVersion, "patch");
	}

	if (!newVersion) {
		throw new Error("Failed to calculate new version");
	}

	return newVersion;
}

// Available steps
const STEPS = [
	"confirm-release",
	"update-version",
	"generate-fern",
	"run-local-build-and-checks",
	"git-commit",
	"git-push",
	"trigger-workflow",
	"validate-reuse-version",
	"run-ci-build-and-checks",
	"build-js-artifacts",
	"publish-sdk",
	"tag-docker",
	"promote-artifacts",
	"push-tag",
	"create-github-release",
	"merge-release",
] as const;

const PHASES = [
	"setup-local",
	"setup-ci",
	"complete-ci",
] as const;

type Step = (typeof STEPS)[number];
type Phase = (typeof PHASES)[number];

// Map phases to individual steps
const PHASE_MAP: Record<Phase, Step[]> = {
	// These steps modify the source code, so they need to be ran & committed
	// locally. CI cannot push commits.
	//
	// run-local-build-and-checks runs type checks, cargo check, and website
	// build to fail fast before committing/pushing. This duplicates setup-ci
	// checks intentionally to catch errors early.
	"setup-local": [
		"confirm-release",
		"update-version",
		"generate-fern",
		"run-local-build-and-checks",
		"git-commit",
		"git-push",
		"trigger-workflow",
	],
	// These steps validate the repository and build JS artifacts before
	// triggering release. Excludes website, mcp-hub, and engine-frontend
	// since they are built separately.
	"setup-ci": ["validate-reuse-version", "run-ci-build-and-checks", "build-js-artifacts"],
	// These steps run after the required artifacts have been successfully built.
	"complete-ci": [
		"publish-sdk",
		"tag-docker",
		"promote-artifacts",
		"push-tag",
		"create-github-release",
	],
};

async function main() {
	// Setup commander
	program
		.name("release")
		.description("Release a new version of Rivet")
		.option("--major", "Bump major version")
		.option("--minor", "Bump minor version")
		.option("--patch", "Bump patch version")
		.option("--version <version>", "Set specific version")
		.option(
			"--override-commit <commit>",
			"Override the commit to pull artifacts from (defaults to current commit)",
		)
		.option(
			"--reuse-engine-version <version-or-commit>",
			"Reuse artifacts and Docker images from a previous version (e.g., 2.0.33) or git revision (e.g., bb7f292)",
		)
		.option("--latest", "Tag version as the latest version (auto-detected by default)")
		.option("--no-latest", "Do not tag version as the latest version")
		.option("--no-validate-git", "Skip git validation (for testing)")
		.option(
			"--only-steps <steps>",
			`Run specific steps (comma-separated). Available: ${STEPS.join(", ")}`,
		)
		.option(
			"--phase <phase>",
			`Run a release phase (comma-separated). Available: ${PHASES.join(", ")}`,
		)
		.parse();

	const opts = program.opts();

	// Parse requested steps
	if (!opts.phase && !opts.onlySteps) {
		throw new Error(
			"Must provide either --phase or --only-steps. Run with --help for more information.",
		);
	}

	if (opts.phase && opts.onlySteps) {
		throw new Error("Cannot use both --phase and --only-steps together");
	}

	const requestedSteps = new Set<Step>();
	if (opts.onlySteps) {
		const steps = opts.onlySteps.split(",").map((s: string) => s.trim());
		for (const step of steps) {
			if (!STEPS.includes(step as Step)) {
				throw new Error(
					`Invalid step: ${step}. Available steps: ${STEPS.join(", ")}`,
				);
			}
			requestedSteps.add(step as Step);
		}
	} else if (opts.phase) {
		const phases = opts.phase.split(",").map((s: string) => s.trim());
		for (const phase of phases) {
			if (!PHASES.includes(phase as Phase)) {
				throw new Error(
					`Invalid phase: ${phase}. Available phases: ${PHASES.join(", ")}`,
				);
			}
			const steps = PHASE_MAP[phase as Phase];
			for (const step of steps) {
				requestedSteps.add(step);
			}
		}
	}

	// Helper function to check if a step should run
	const shouldRunStep = (step: Step): boolean => {
		return requestedSteps.has(step);
	};

	// Get version from arguments or calculate based on flags
	const version = await getVersionFromArgs({
		version: opts.version,
		major: opts.major,
		minor: opts.minor,
		patch: opts.patch,
	});

	assert(
		/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$/.test(
			version,
		),
		"version must be a valid semantic version",
	);

	// Automatically determine if this should be tagged as latest
	// Can be overridden by --latest or --no-latest flags
	let isLatest: boolean;
	if (opts.latest !== undefined) {
		// User explicitly set the flag
		isLatest = opts.latest;
	} else {
		// Auto-determine based on version
		isLatest = await shouldTagAsLatest(version);
		console.log(`Auto-determined latest flag: ${isLatest} (version: ${version})`);
	}

	// Setup opts
	let commit: string;
	if (opts.overrideCommit) {
		// Manually override commit
		commit = opts.overrideCommit;
	} else {
		// Read commit
		const result = await $`git rev-parse HEAD`;
		commit = result.stdout.trim();
	}

	const releaseOpts: ReleaseOpts = {
		root: ROOT_DIR,
		version: version,
		latest: isLatest,
		commit,
		reuseEngineVersion: opts.reuseEngineVersion,
	};

	if (releaseOpts.commit.length == 40) {
		releaseOpts.commit = releaseOpts.commit.slice(0, 7);
	}

	assertEquals(releaseOpts.commit.length, 7, "must use 8 char short commit");

	if (opts.validateGit && !shouldRunStep("run-ci-build-and-checks")) {
		// HACK: Skip setup-ci because for some reason there's changes in the setup step but only in GitHub Actions
		await validateGit(releaseOpts);
	}

	if (shouldRunStep("confirm-release")) {
		console.log("==> Release Confirmation");
		console.log(`\nRelease Details:`);
		console.log(`  Version: ${releaseOpts.version}`);
		console.log(`  Latest: ${releaseOpts.latest}`);
		console.log(`  Commit: ${releaseOpts.commit}`);
		if (releaseOpts.reuseEngineVersion) {
			console.log(`  Reusing engine version: ${releaseOpts.reuseEngineVersion}`);
		}

		// Get current branch
		const branchResult = await $`git rev-parse --abbrev-ref HEAD`;
		const branch = branchResult.stdout.trim();
		console.log(`  Branch: ${branch}`);

		// Get and display recent versions
		const allVersions = await getAllGitVersions();

		if (allVersions.length > 0) {
			// Find the latest stable version (excluding prereleases)
			const stableVersions = allVersions.filter(v => {
				const parsed = semver.parse(v);
				return parsed && parsed.prerelease.length === 0;
			});
			const latestStableVersion = stableVersions[0] || null;

			console.log(`\nRecent versions:`);
			const recentVersions = allVersions.slice(0, 10);
			for (const version of recentVersions) {
				const isLatest = version === latestStableVersion;
				const marker = isLatest ? " (latest)" : "";
				console.log(`  - ${version}${marker}`);
			}
		}

		// Prompt for confirmation
		const readline = await import("node:readline");
		const rl = readline.createInterface({
			input: process.stdin,
			output: process.stdout,
		});

		const answer = await new Promise<string>((resolve) => {
			rl.question("\nProceed with release? (yes/no): ", resolve);
		});
		rl.close();

		if (answer.toLowerCase() !== "yes" && answer.toLowerCase() !== "y") {
			console.log("Release cancelled");
			process.exit(0);
		}

		console.log("✅ Release confirmed");
	}

	if (shouldRunStep("update-version")) {
		console.log("==> Updating Version");
		await updateVersion(releaseOpts);
	}

	if (shouldRunStep("generate-fern")) {
		console.log("==> Generating Fern");
		await $({ stdio: "inherit" })`./scripts/fern/gen.sh`;
	}

	if (shouldRunStep("run-local-build-and-checks")) {
		console.log("==> Running Local Build and Checks");
		await runBuildAndChecks(releaseOpts, { ci: false });
	}

	if (shouldRunStep("git-commit")) {
		assert(opts.validateGit, "cannot commit without git validation");
		console.log("==> Committing Changes");
		await $({ stdio: "inherit" })`git add .`;
		await $({
			stdio: "inherit",
			shell: true,
		})`git commit --allow-empty -m "chore(release): update version to ${releaseOpts.version}"`;
	}

	if (shouldRunStep("git-push")) {
		assert(opts.validateGit, "cannot push without git validation");
		console.log("==> Pushing Commits");
		const branchResult = await $`git rev-parse --abbrev-ref HEAD`;
		const branch = branchResult.stdout.trim();
		if (branch === "main") {
			// Push on main
			await $({ stdio: "inherit" })`git push`;
		} else {
			// Modify current branch
			await $({ stdio: "inherit" })`gt submit --force --no-edit --publish`;
		}
	}

	if (shouldRunStep("trigger-workflow")) {
		console.log("==> Triggering Workflow");
		const branchResult = await $`git rev-parse --abbrev-ref HEAD`;
		const branch = branchResult.stdout.trim();
		const latestFlag = releaseOpts.latest ? "true" : "false";

		// Build workflow command
		let workflowCmd = `gh workflow run .github/workflows/release.yaml -f version=${releaseOpts.version} -f latest=${latestFlag}`;
		if (releaseOpts.reuseEngineVersion) {
			workflowCmd += ` -f reuse_engine_version=${releaseOpts.reuseEngineVersion}`;
		}
		workflowCmd += ` --ref ${branch}`;

		await $({ stdio: "inherit", shell: true })`${workflowCmd}`;

		// Get repository info and print workflow link
		const repoResult = await $`gh repo view --json nameWithOwner -q .nameWithOwner`;
		const repo = repoResult.stdout.trim();
		console.log(`\nWorkflow triggered: https://github.com/${repo}/actions/workflows/release.yaml`);
		console.log(`View all runs: https://github.com/${repo}/actions`);
	}

	if (shouldRunStep("validate-reuse-version")) {
		if (releaseOpts.reuseEngineVersion) {
			console.log("==> Validating Reuse Version");
			await validateReuseVersion(releaseOpts.reuseEngineVersion);
		}
	}

	if (shouldRunStep("run-ci-build-and-checks")) {
		console.log("==> Running CI Build and Checks");
		await runBuildAndChecks(releaseOpts, { ci: true });
	}

	if (shouldRunStep("build-js-artifacts")) {
		console.log("==> Building JS Artifacts");
		await buildJsArtifacts(releaseOpts);
	}

	if (shouldRunStep("publish-sdk")) {
		console.log("==> Publishing SDKs");
		await publishSdk(releaseOpts);
	}

	if (shouldRunStep("tag-docker")) {
		console.log("==> Tagging Docker");
		await tagDocker(releaseOpts);
	}

	if (shouldRunStep("promote-artifacts")) {
		console.log("==> Promoting Artifacts");
		await promoteArtifacts(releaseOpts);
	}

	if (shouldRunStep("push-tag")) {
		console.log("==> Pushing Tag");
		await createAndPushTag(releaseOpts);
	}

	if (shouldRunStep("create-github-release")) {
		console.log("==> Creating GitHub Release");
		await createGitHubRelease(releaseOpts);
	}

	console.log("==> Complete");
}

main().catch((err) => {
	console.error(err);
	process.exit(1);
});
