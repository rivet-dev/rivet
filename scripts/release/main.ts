#!/usr/bin/env tsx

import * as fs from "node:fs/promises";
import * as path from "node:path";
import * as url from "node:url";
import { $ } from "execa";
import { program } from "commander";
import * as semver from "semver";
import { updateArtifacts } from "./artifacts";
import { tagDocker } from "./docker";
import {
	createAndPushTag,
	createGitHubRelease,
	validateGit,
} from "./git";
import { configureReleasePlease } from "./release_please";
import { publishSdk } from "./sdk";
import { updateVersion } from "./update_version";

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
const ROOT_DIR = path.resolve(__dirname, "..", "..");

function assert(condition: any, message?: string): asserts condition {
	if (!condition) {
		throw new Error(message || "Assertion failed");
	}
}

function assertEquals<T>(actual: T, expected: T, message?: string): void {
	if (actual !== expected) {
		throw new Error(message || `Expected ${expected}, got ${actual}`);
	}
}

function assertExists<T>(
	value: T | null | undefined,
	message?: string,
): asserts value is T {
	if (value === null || value === undefined) {
		throw new Error(message || "Value does not exist");
	}
}

export interface ReleaseOpts {
	root: string;
	version: string;
	latest: boolean;
	/** Commit to publish release for. */
	commit: string;
}

async function getCurrentVersion(): Promise<string> {
	// Get version from the main rivetkit package
	const packageJsonPath = path.resolve(
		ROOT_DIR,
		"rivetkit-typescript/packages/rivetkit/package.json",
	);
	const packageJson = JSON.parse(await fs.readFile(packageJsonPath, "utf-8"));
	return packageJson.version;
}

async function runTypeCheck(opts: ReleaseOpts) {
	console.log("Checking types...");
	try {
		// --force to skip cache in case of Turborepo bugs
		await $({ cwd: opts.root })`pnpm check-types --force`;
		console.log("✅ Type check passed");
	} catch (err) {
		console.error("❌ Type check failed");
		throw err;
	}
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

	// Get current version and calculate new one
	const currentVersion = await getCurrentVersion();
	console.log(`Current version: ${currentVersion}`);

	let newVersion: string | null = null;

	if (opts.major) {
		newVersion = semver.inc(currentVersion, "major");
	} else if (opts.minor) {
		newVersion = semver.inc(currentVersion, "minor");
	} else if (opts.patch) {
		newVersion = semver.inc(currentVersion, "patch");
	}

	if (!newVersion) {
		throw new Error("Failed to calculate new version");
	}

	return newVersion;
}

// Available steps
const STEPS = [
	"update-version",
	"generate-fern",
	"git-commit",
	"configure-release-please",
	"git-push",
	"run-type-check",
	"publish-sdk",
	"tag-docker",
	"update-artifacts",
	"push-tag",
	"create-github-release",
	"merge-release",
] as const;

const BATCH_STEPS = [
	"setup-local",
	"setup-ci",
	"complete-ci",
] as const;

type Step = (typeof STEPS)[number];
type BatchStep = (typeof BATCH_STEPS)[number];

// Map batch steps to individual steps
const BATCH_STEP_MAP: Record<BatchStep, Step[]> = {
	"setup-local": [
		"update-version",
		"generate-fern",
		"git-commit",
		"configure-release-please",
		"git-push",
	],
	"setup-ci": ["run-type-check"],
	"complete-ci": [
		"publish-sdk",
		"tag-docker",
		"update-artifacts",
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
		.option("--commit <commit>", "Commit to publish release for")
		.option("--latest", "Tag version as the latest version", true)
		.option("--no-latest", "Do not tag version as the latest version")
		.option("--no-validate-git", "Skip git validation (for testing)")
		.option(
			"--steps <steps>",
			`Run specific steps (comma-separated). Available: ${STEPS.join(", ")}`,
		)
		.option(
			"--batch-steps <steps>",
			`Run batch steps (comma-separated). Available: ${BATCH_STEPS.join(", ")}`,
		)
		.parse();

	const opts = program.opts();

	// Parse steps from flags
	const requestedSteps = new Set<Step>();

	// Add steps from --steps flag
	if (opts.steps) {
		const steps = opts.steps.split(",").map((s: string) => s.trim());
		for (const step of steps) {
			if (!STEPS.includes(step as Step)) {
				throw new Error(
					`Invalid step: ${step}. Available steps: ${STEPS.join(", ")}`,
				);
			}
			requestedSteps.add(step as Step);
		}
	}

	// Add steps from --batch-steps flag
	if (opts.batchSteps) {
		const batchSteps = opts.batchSteps
			.split(",")
			.map((s: string) => s.trim());
		for (const batchStep of batchSteps) {
			if (!BATCH_STEPS.includes(batchStep as BatchStep)) {
				throw new Error(
					`Invalid batch step: ${batchStep}. Available batch steps: ${BATCH_STEPS.join(", ")}`,
				);
			}
			const steps = BATCH_STEP_MAP[batchStep as BatchStep];
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

	// Setup opts
	let commit: string;
	if (opts.commit) {
		// Manually override commit
		commit = opts.commit;
	} else {
		// Read commit
		const result = await $`git rev-parse HEAD`;
		commit = result.stdout.trim();
	}

	const releaseOpts: ReleaseOpts = {
		root: ROOT_DIR,
		version: version,
		latest: opts.latest,
		commit,
	};

	if (releaseOpts.commit.length == 40) {
		releaseOpts.commit = releaseOpts.commit.slice(0, 7);
	}

	assertEquals(releaseOpts.commit.length, 7, "must use 8 char short commit");

	if (opts.validateGit && !shouldRunStep("run-type-check")) {
		// HACK: Skip setup-ci because for some reason there's changes in the setup step but only in GitHub Actions
		await validateGit(releaseOpts);
	}

	if (shouldRunStep("update-version")) {
		console.log("==> Updating Version");
		await updateVersion(releaseOpts);
	}

	if (shouldRunStep("generate-fern")) {
		console.log("==> Generating Fern");
		await $`./scripts/fern/gen.sh`;
	}

	if (shouldRunStep("git-commit")) {
		assert(opts.validateGit, "cannot commit without git validation");
		console.log("==> Committing Changes");
		await $`git add .`;
		await $({
			shell: true,
		})`git commit --allow-empty -m "chore(release): update version to ${releaseOpts.version}"`;
	}

	if (shouldRunStep("configure-release-please")) {
		assert(
			opts.validateGit,
			"cannot configure release please without git validation",
		);
		console.log("==> Configuring Release Please");
		await configureReleasePlease(releaseOpts);
	}

	if (shouldRunStep("git-push")) {
		assert(opts.validateGit, "cannot push without git validation");
		console.log("==> Pushing Commits");
		const branchResult = await $`git rev-parse --abbrev-ref HEAD`;
		const branch = branchResult.stdout.trim();
		if (branch === "main") {
			// Push on main
			await $`git push`;
		} else {
			// Modify current branch
			await $`gt submit --force --no-edit --publish`;
		}
	}

	if (shouldRunStep("run-type-check")) {
		console.log("==> Running Type Check");
		await runTypeCheck(releaseOpts);
	}

	if (shouldRunStep("publish-sdk")) {
		console.log("==> Publishing SDKs");
		await publishSdk(releaseOpts);
	}

	if (shouldRunStep("tag-docker")) {
		console.log("==> Tagging Docker");
		await tagDocker(releaseOpts);
	}

	if (shouldRunStep("update-artifacts")) {
		console.log("==> Updating Artifacts");
		await updateArtifacts(releaseOpts);
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
