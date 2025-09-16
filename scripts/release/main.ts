#!/usr/bin/env tsx

import * as path from "node:path";
import * as fs from "node:fs";
import * as url from "node:url";
import minimist from "minimist";
import { $ } from "execa";
import { publishSdk } from "./sdk.ts";
import { updateVersion } from "./update_version.ts";
import { configureReleasePlease } from "./release_please.ts";
import { validateGit } from "./git.ts";
import { tagDocker } from "./docker.ts";
import { updateArtifacts } from "./artifacts.ts";

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

function assertExists<T>(value: T | null | undefined, message?: string): asserts value is T {
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

async function main() {
	// Parse args:
	// - latest = tag version as the latest version
	// - noValidateGit = used for testing without using the main branch
	// - setup & complete = run all pre-build or post-build steps, used in CI for batch jbos
	const args = minimist(process.argv.slice(2), {
		boolean: [
			// Config
			"latest",
			"noValidateGit",

			// Granular steps
			"updateVersion",
			"generateFern",
			"gitCommit",
			"configureReleasePlease",
			"gitPush",
			"publishSdk",
			"tagDocker",
			"updateArtifacts",
			"mergeRelease",

			// Batch steps
			"setupLocal",  // Makes changes to repo & pushes it (we can't push commits from CI that can trigger Release Please & other CI actions)
			"setupCi",  // Publishes packages (has access to NPM creds)
			"completeCi",  // Tags binaries & Docker as latest (has access to Docker & S3 creds)
		],
		negatable: ["latest"],
		string: ["version", "commit"],
		default: {
			latest: true,
		},
	});
	assertExists(args.version);

	assert(
		/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$/.test(
			args.version,
		),
		"version must be a valid semantic version",
	);

	// Setup opts
	let commit: string;
	if (args.commit) {
		// Manually override commit
		commit = args.commit;
	} else {
		// Read commit
		const result = await $`git rev-parse HEAD`;
		commit = result.stdout.trim();
	}

	const opts: ReleaseOpts = {
		root: ROOT_DIR,
		version: args.version,
		latest: args.latest,
		commit,
	};

	if (opts.commit.length == 40) {
		opts.commit = opts.commit.slice(0, 7);
	}

	assertEquals(opts.commit.length, 7, "must use 8 char short commit");

	if (!args.noValidateGit && !args.setupCi) {
		// HACK: Skip setupCi because for some reason there's changes in the setup step but only in GitHub Actions
		await validateGit(opts);
	}

	if (args.updateVersion || args.setupLocal) {
		console.log("==> Updating Version");
		await updateVersion(opts);
	}

	if (args.generateFern || args.setupLocal) {
		console.log("==> Generating Fern");
		await $`./scripts/fern/gen.sh`;
	}

	if (args.gitCommit || args.setupLocal) {
		assert(!args.noValidateGit, "cannot commit without git validation");
		console.log("==> Committing Changes");
		await $`git add .`;
		await $({ shell: true })`git commit --allow-empty -m "chore(release): update version to ${opts.version}"`;
	}

	if (args.configureReleasePlease || args.setupLocal) {
		assert(!args.noValidateGit, "cannot configure release please without git validation");
		console.log("==> Configuring Release Please");
		await configureReleasePlease(opts);
	}

	if (args.gitPush || args.setupLocal) {
		assert(!args.noValidateGit, "cannot push without git validation");
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

	// TODO: Currently using pkg.pr.new
	// if (args.publishSdk || args.setupCi) {
	// 	console.log("==> Publishing SDKs");
	// 	await publishSdk(opts);
	// }

	if (args.tagDocker || args.completeCi) {
		console.log("==> Tagging Docker");
		await tagDocker(opts);
	}

	if (args.updateArtifacts || args.completeCi) {
		console.log("==> Updating Artifacts");
		await updateArtifacts(opts);
	}

	console.log("==> Complete");
}

main().catch((err) => {
	console.error(err);
	process.exit(1);
});

