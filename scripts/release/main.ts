#!/usr/bin/env -S deno run -A

import { resolve } from "jsr:@std/path";
import { assert, assertEquals, assertExists } from "jsr:@std/assert";
import { publishSdk } from "./sdk.ts";
import { updateVersion } from "./update_version.ts";
import { configureReleasePlease } from "./release_please.ts";
import { getCommit, validateGit } from "./git.ts";
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
			"format",
			"updateVersion",
			"generateFern",
			"gitCommit",
			"configureReleasePlease",
			"gitPush",
			"publishSdk",
			"tagDocker",
			"updateArtifacts",

			// Batch steps
			"setup",
			"complete",
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
		commit = await getCommit();
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

	if (!args.noValidateGit) {
		await validateGit(opts);
	}

	if (args.format || args.setup) {
		$.logStep("Formatting", "");
		await $.logGroup(async () => {
			await $`cargo fmt`;
		});
	}

	if (args.updateVersion || args.setup) {
		$.logStep("Updating Version", "");
		await $.logGroup(async () => {
			await updateVersion(opts);
		});
	}

	if (args.generateFern || args.setup) {
		$.logStep("Generating Fern", "");
		await $.logGroup(async () => {
			await $`./scripts/fern/gen.sh`;
		});
	}

	if (args.publishSdk || args.setup) {
		$.logStep("Publishing SDKs", "");
		await $.logGroup(async () => {
			await publishSdk(opts);
		});
	}

	if (args.gitCommit || args.setup) {
		assert(!args.noValidateGit, "cannot commit without git validation");
		$.logStep("Committing Changes", "");
		await $.logGroup(async () => {
			await $`git commit --allow-empty -m ${`chore(release): update version to ${opts.version}`}`;
		});
	}

	if (args.configureReleasePlease || args.setup) {
		assert(!args.noValidateGit, "cannot configure release please without git validation");
		$.logStep("Configuring Release Please", "");
		await $.logGroup(async () => {
			await configureReleasePlease(opts);
		});
	}

	if (args.gitPush || args.setup) {
		assert(!args.noValidateGit, "cannot push without git validation");
		$.logStep("Pushing Commits", "");
		await $.logGroup(async () => {
			await $`git push`;
		});
	}

	if (args.tagDocker || args.complete) {
		$.logStep("Tagging Docker", "");
		await $.logGroup(async () => {
			await tagDocker(opts);
		});
	}

	if (args.updateArtifacts || args.complete) {
		$.logStep("Updating Artifacts", "");
		await $.logGroup(async () => {
			await updateArtifacts(opts);
		});
	}

	$.logStep("Complete");
	$.logWarn("Important", "Make sure to release the Release Please PR");
}

if (import.meta.main) {
	main();
}

