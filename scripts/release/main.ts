#!/usr/bin/env -S deno run -A

import { resolve } from "jsr:@std/path";
import { assert, assertEquals, assertExists } from "jsr:@std/assert";
import { publishSdk } from "./sdk.ts";
import { updateVersion } from "./update_version.ts";
import { configureReleasePlease } from "./release_please.ts";
import { validateGit } from "./git.ts";
import { parseArgs } from "jsr:@std/cli";
import $ from "dax";
import { tagDocker } from "./docker.ts";
import { updateArtifacts } from "./artifacts.ts";

const ROOT_DIR = resolve(import.meta.dirname!, "..", "..");

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
	const args = parseArgs(Deno.args, {
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
		commit = await $`git rev-parse HEAD`.text();
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
		$.logStep("Updating Version", "");
		await $.logGroup(async () => {
			await updateVersion(opts);
		});
	}

	if (args.generateFern || args.setupLocal) {
		$.logStep("Generating Fern", "");
		await $.logGroup(async () => {
			await $`./scripts/fern/gen.sh`;
		});
	}

	if (args.gitCommit || args.setupLocal) {
		assert(!args.noValidateGit, "cannot commit without git validation");
		$.logStep("Committing Changes", "");
		await $.logGroup(async () => {
			await $`git add .`;
			await $`git commit --allow-empty -m ${`chore(release): update version to ${opts.version}`}`;
		});
	}

	if (args.configureReleasePlease || args.setupLocal) {
		assert(!args.noValidateGit, "cannot configure release please without git validation");
		$.logStep("Configuring Release Please", "");
		await $.logGroup(async () => {
			await configureReleasePlease(opts);
		});
	}

	if (args.gitPush || args.setupLocal) {
		assert(!args.noValidateGit, "cannot push without git validation");
		$.logStep("Pushing Commits", "");
		await $.logGroup(async () => {
			const branch = (await $`git rev-parse --abbrev-ref HEAD`.stdout("piped")).stdout.trim();
			if (branch === "main") {
				// Push on main
				await $`git push`;
			} else {
				// Modify current branch
				await $`gt submit --force --no-edit --publish`;
			}
		});
	}

	if (args.publishSdk || args.setupCi) {
		$.logStep("Publishing SDKs", "");
		await $.logGroup(async () => {
			await publishSdk(opts);
		});
	}

	if (args.tagDocker || args.completeCi) {
		$.logStep("Tagging Docker", "");
		await $.logGroup(async () => {
			await tagDocker(opts);
		});
	}

	if (args.updateArtifacts || args.completeCi) {
		$.logStep("Updating Artifacts", "");
		await $.logGroup(async () => {
			await updateArtifacts(opts);
		});
	}

	$.logStep("Complete");
}

if (import.meta.main) {
	main();
}

