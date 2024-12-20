#!/usr/bin/env -S deno run -A

import { copy } from "jsr:@std/fs";
import { resolve } from "jsr:@std/path";
import { assert, assertEquals, assertExists } from "jsr:@std/assert";
import { S3Bucket } from "https://deno.land/x/s3@0.5.0/mod.ts";
import { publishSdk } from "./sdk.ts";
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
	// Parse args
	const args = parseArgs(Deno.args, {
		boolean: ["latest", "publishSdk", "tagDocker", "updateArtifacts", "configureReleasePlease"],
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

	await validateGit(opts);

	// Determine which steps to run
	const runAllSteps = !args.publishSdk && !args.tagDocker && !args.updateArtifacts && !args.configureReleasePlease;

	if (args.publishSdk || runAllSteps) {
		$.logStep("Publishing SDKs", "");
		await $.logGroup(async () => {
			await publishSdk(opts);
		});
	}

	if (args.tagDocker || runAllSteps) {
		$.logStep("Tagging Docker", "");
		await $.logGroup(async () => {
			await tagDocker(opts);
		});
	}

	if (args.updateArtifacts || runAllSteps) {
		$.logStep("Updating Artifacts", "");
		await $.logGroup(async () => {
			await updateArtifacts(opts);
		});
	}

	if (args.configureReleasePlease || runAllSteps) {
		$.logStep("Configuring Release Please", "");
		await $.logGroup(async () => {
			await configureReleasePlease(opts);
		});
	}

	$.logStep("Complete");
	$.logWarn("Important", "Make sure to release the Release Please PR");
}

if (import.meta.main) {
	main();
}

// More steps:
// - Update version
//	- Default version in initiated project
//	- Version for docs
//	- Install instructions in readme
//	- Cargo.toml
// - Docker images & docker monolith (tag with version & latest)
// - Copy release binaries to latest tag
// - Create rename commit for release please (release please will tag repo)
// - Commit changes for version update in JSR/NPM
// - Merge release please
