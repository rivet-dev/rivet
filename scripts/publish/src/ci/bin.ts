#!/usr/bin/env tsx
/**
 * CI entrypoint. Every workflow step calls exactly one subcommand.
 *
 * Each subcommand is a pure function of its flags — nothing orchestrates
 * other subcommands. The GitHub Actions workflow is the orchestrator.
 *
 * Subcommands accept inputs via flags AND will fall back to re-resolving
 * the `PublishContext` from env vars when flags aren't passed. The workflow
 * uses the `context-output` subcommand to resolve once and pin values as
 * job outputs, then passes those outputs to each subsequent step as flags.
 */
import { readFileSync } from "node:fs";
import { dirname, join, resolve as resolvePath } from "node:path";
import { fileURLToPath } from "node:url";
import { Command } from "commander";
import { $ } from "execa";
import {
	resolveContext,
	writeContextToGithubOutput,
	type PublishContext,
	type Trigger,
} from "../lib/context.js";
import {
	createMultiArchManifests,
	retagManifestsToVersion,
} from "../lib/docker.js";
import {
	createGhRelease,
	tagAndPush,
} from "../lib/git.js";
import { scoped } from "../lib/logger.js";
import { publishAll } from "../lib/npm.js";
import {
	copyPrefix,
	uploadDir,
	uploadFile,
	uploadInstallScripts,
} from "../lib/r2.js";
import { bumpPackageJsons } from "../lib/version.js";

const log = scoped("ci");

function findRepoRoot(): string {
	if (process.env.GITHUB_WORKSPACE) return process.env.GITHUB_WORKSPACE;
	let dir = dirname(fileURLToPath(import.meta.url));
	for (let i = 0; i < 10; i++) {
		try {
			const ws = join(dir, "pnpm-workspace.yaml");
			readFileSync(ws, "utf-8");
			return dir;
		} catch {
			dir = dirname(dir);
		}
	}
	throw new Error("could not locate repo root");
}

const program = new Command();
program.name("ci").description("CI subcommands for the publish flow");

// ---------------------------------------------------------------------------
// context-output — resolve once, write to $GITHUB_OUTPUT for downstream steps
// ---------------------------------------------------------------------------
program
	.command("context-output")
	.description("Resolve publish context and write to $GITHUB_OUTPUT")
	.option("--trigger <trigger>", "Override trigger (pr|main|release)")
	.option("--version <version>", "Override version")
	.option("--latest <bool>", "Override latest")
	.option("--pr-number <number>", "Override PR number")
	.action(async (opts) => {
		const overrides: Parameters<typeof resolveContext>[0] = {};
		if (opts.trigger) overrides.trigger = opts.trigger as Trigger;
		if (opts.version) overrides.version = opts.version;
		if (opts.latest !== undefined) {
			overrides.latest = opts.latest === "true";
		}
		if (opts.prNumber) overrides.prNumber = Number(opts.prNumber);
		const ctx = await resolveContext(overrides);
		log.info(
			`resolved: trigger=${ctx.trigger} version=${ctx.version} npm_tag=${ctx.npmTag} sha=${ctx.sha} latest=${ctx.latest}${ctx.prNumber !== undefined ? ` pr=${ctx.prNumber}` : ""}`,
		);
		writeContextToGithubOutput(ctx);
	});

// ---------------------------------------------------------------------------
// bump-versions — rewrite package.jsons to a version (+ inject optionalDeps)
// ---------------------------------------------------------------------------
program
	.command("bump-versions")
	.description("Rewrite every publishable package.json to the given version")
	.option("--version <version>", "Version to write (defaults to resolved context)")
	.option(
		"--version-only",
		"Only rewrite package.json version fields without publish-time dependency injection",
	)
	.option("--dry-run", "Do not write, only report")
	.action(async (opts) => {
		const repoRoot = findRepoRoot();
		const version =
			opts.version ?? (await resolveContext()).version;
		await bumpPackageJsons(repoRoot, version, {
			dryRun: !!opts.dryRun,
			versionOnly: !!opts.versionOnly,
		});
	});

// ---------------------------------------------------------------------------
// publish-npm — parallel npm publish with retries
// ---------------------------------------------------------------------------
program
	.command("publish-npm")
	.description("Publish all discovered packages to npm")
	.option("--tag <tag>", "npm dist-tag (defaults to resolved context)")
	.option("--parallel <n>", "Max simultaneous publishes", "16")
	.option("--retries <n>", "Retries per package", "3")
	.option("--release-mode", "Fail if every package is already published")
	.action(async (opts) => {
		const repoRoot = findRepoRoot();
		let tag: string = opts.tag;
		let releaseMode = !!opts.releaseMode;
		if (!tag || releaseMode === undefined) {
			const ctx = await resolveContext();
			tag = tag ?? ctx.npmTag;
			if (opts.releaseMode === undefined) {
				releaseMode = ctx.trigger === "release";
			}
		}
		await publishAll(repoRoot, {
			tag,
			parallel: Number(opts.parallel),
			retries: Number(opts.retries),
			releaseMode,
		});
	});

// ---------------------------------------------------------------------------
// upload-r2 — upload a directory to rivet/{sha}/engine/ (or a custom dest)
// ---------------------------------------------------------------------------
program
	.command("upload-r2")
	.description("Upload an artifact directory to R2")
	.requiredOption("--source <dir>", "Local directory to upload")
	.option("--sha <sha>", "Short sha (defaults to resolved context)")
	.option("--name <name>", "R2 sub-path name", "engine")
	.action(async (opts) => {
		const sha = opts.sha ?? (await resolveContext()).sha;
		const prefix = `rivet/${sha}/${opts.name}/`;
		await uploadDir(opts.source, prefix);
	});

// ---------------------------------------------------------------------------
// copy-r2 — copy rivet/{sha}/engine/ to rivet/{version}/engine/ (+latest)
// ---------------------------------------------------------------------------
program
	.command("copy-r2")
	.description("Copy R2 artifacts from {sha} to {version} (+latest)")
	.option("--sha <sha>", "Source sha (defaults to resolved context)")
	.option("--version <version>", "Target version (defaults to resolved context)")
	.option("--latest <bool>", "Also copy to /latest/ (defaults to resolved context)")
	.option("--name <name>", "R2 sub-path name", "engine")
	.action(async (opts) => {
		const ctx = await resolveContext();
		const sha: string = opts.sha ?? ctx.sha;
		const version: string = opts.version ?? ctx.version;
		const latest =
			opts.latest !== undefined ? opts.latest === "true" : ctx.latest;
		const source = `rivet/${sha}/${opts.name}/`;
		await copyPrefix(source, `rivet/${version}/${opts.name}/`);
		if (latest) {
			await copyPrefix(source, `rivet/latest/${opts.name}/`);
		}
	});

// ---------------------------------------------------------------------------
// upload-install-scripts — template install.sh/ps1 with the release version
// ---------------------------------------------------------------------------
program
	.command("upload-install-scripts")
	.description("Upload install.sh/install.ps1 templated with the version")
	.requiredOption("--scripts-dir <dir>", "Directory containing install.sh and install.ps1")
	.option("--version <version>", "Version to template (defaults to context)")
	.option("--latest <bool>", "Also upload to /latest/")
	.action(async (opts) => {
		const ctx = await resolveContext();
		const version: string = opts.version ?? ctx.version;
		const latest =
			opts.latest !== undefined ? opts.latest === "true" : ctx.latest;
		const scripts = [
			resolvePath(opts.scriptsDir, "install.sh"),
			resolvePath(opts.scriptsDir, "install.ps1"),
		];
		await uploadInstallScripts(scripts, version);
		if (latest) await uploadInstallScripts(scripts, "latest");
	});

// ---------------------------------------------------------------------------
// upload-devtools — build & upload @rivetkit/devtools to R2 (release only)
// ---------------------------------------------------------------------------
program
	.command("upload-devtools")
	.description("Build @rivetkit/devtools and upload dist to R2")
	.option("--sha <sha>", "Short sha (defaults to resolved context)")
	.option("--version <version>", "Version (defaults to resolved context)")
	.option("--latest <bool>", "Also copy to /latest/ (defaults to resolved context)")
	.action(async (opts) => {
		const repoRoot = findRepoRoot();
		const ctx = await resolveContext();
		const sha = opts.sha ?? ctx.sha;
		const version: string = opts.version ?? ctx.version;
		const latest =
			opts.latest !== undefined ? opts.latest === "true" : ctx.latest;
		log.info("building @rivetkit/devtools");
		await $({ stdio: "inherit", cwd: repoRoot })`pnpm build -F @rivetkit/devtools`;
		const dist = resolvePath(
			repoRoot,
			"rivetkit-typescript/packages/devtools/dist",
		);
		// Upload to commit path first, then copy to version/latest (matches the
		// R2 engine path shape so both can be promoted the same way).
		await uploadDir(dist, `rivet/${sha}/devtools/`);
		await copyPrefix(
			`rivet/${sha}/devtools/`,
			`rivet/${version}/devtools/`,
		);
		if (latest) {
			await copyPrefix(
				`rivet/${sha}/devtools/`,
				"rivet/latest/devtools/",
			);
		}
	});

// ---------------------------------------------------------------------------
// docker-manifest — create multi-arch manifest for {sha}
// ---------------------------------------------------------------------------
program
	.command("docker-manifest")
	.description("Create multi-arch Docker manifests for the commit sha")
	.option("--sha <sha>", "Short sha (defaults to resolved context)")
	.action(async (opts) => {
		const sha = opts.sha ?? (await resolveContext()).sha;
		await createMultiArchManifests(sha);
	});

// ---------------------------------------------------------------------------
// docker-retag — retag {sha} manifests to {version} (+latest)
// ---------------------------------------------------------------------------
program
	.command("docker-retag")
	.description("Retag multi-arch manifests from {sha} to {version}")
	.option("--sha <sha>", "Source sha (defaults to resolved context)")
	.option("--version <version>", "Target version (defaults to resolved context)")
	.option("--latest <bool>", "Also tag as latest (defaults to resolved context)")
	.action(async (opts) => {
		const ctx = await resolveContext();
		const sha: string = opts.sha ?? ctx.sha;
		const version: string = opts.version ?? ctx.version;
		const latest =
			opts.latest !== undefined ? opts.latest === "true" : ctx.latest;
		await retagManifestsToVersion(sha, version, latest);
	});

// ---------------------------------------------------------------------------
// git-tag — force-create and push v{version}
// ---------------------------------------------------------------------------
program
	.command("git-tag")
	.description("Create and force-push v{version} tag")
	.option("--version <version>", "Version (defaults to resolved context)")
	.action(async (opts) => {
		const version =
			opts.version ?? (await resolveContext()).version;
		await tagAndPush(version);
	});

// ---------------------------------------------------------------------------
// gh-release — create or update GitHub release for the version
// ---------------------------------------------------------------------------
program
	.command("gh-release")
	.description("Create or update GitHub release")
	.option("--version <version>", "Version (defaults to resolved context)")
	.action(async (opts) => {
		const version =
			opts.version ?? (await resolveContext()).version;
		await createGhRelease(version);
	});

// ---------------------------------------------------------------------------
// comment-pr — upsert the "Preview packages published to npm" PR comment
// ---------------------------------------------------------------------------
program
	.command("comment-pr")
	.description("Upsert the preview PR comment with install instructions")
	.option("--pr-number <number>", "PR number (defaults to context)")
	.option("--version <version>", "Version (defaults to context)")
	.option("--tag <tag>", "npm dist-tag (defaults to context)")
	.action(async (opts) => {
		const ctx = await resolveContext();
		const prNumber = opts.prNumber ? Number(opts.prNumber) : ctx.prNumber;
		const version: string = opts.version ?? ctx.version;
		const tag: string = opts.tag ?? ctx.npmTag;
		if (typeof prNumber !== "number") {
			throw new Error("comment-pr requires a PR number");
		}
		const repo = process.env.GITHUB_REPOSITORY;
		if (!repo) {
			throw new Error("GITHUB_REPOSITORY env var not set");
		}
		const marker = "Preview packages published to npm";
		const body = [
			`## ${marker}`,
			"",
			"Install with:",
			"```sh",
			`npm install rivetkit@${tag}`,
			"```",
			"",
			`All packages published as \`${version}\` with tag \`${tag}\`.`,
			"",
			"Engine binary is shipped via `@rivetkit/engine-cli` (platforms: linux-x64-musl, linux-arm64-musl, darwin-x64, darwin-arm64). `rivetkit` resolves it automatically at runtime.",
			"",
			"Docker images:",
			"```sh",
			`docker pull rivetdev/engine:slim-${ctx.sha}`,
			`docker pull rivetdev/engine:full-${ctx.sha}`,
			"```",
			"",
			"<details>",
			"<summary>Individual packages</summary>",
			"",
			"```sh",
			`npm install rivetkit@${tag}`,
			`npm install @rivetkit/react@${tag}`,
			`npm install @rivetkit/rivetkit-native@${tag}`,
			`npm install @rivetkit/sqlite-wasm@${tag}`,
			`npm install @rivetkit/workflow-engine@${tag}`,
			"```",
			"",
			"</details>",
		].join("\n");

		// Find existing comment by marker, update if present, else create.
		// `gh api -f body=<value>` sends string fields directly (no @file needed).
		const { stdout: commentsJson } = await $({
			env: process.env as Record<string, string>,
		})`gh api repos/${repo}/issues/${String(prNumber)}/comments --paginate`;
		const comments = JSON.parse(commentsJson) as Array<{
			id: number;
			body: string;
		}>;
		const existing = comments.find((c) => c.body.includes(marker));
		if (existing) {
			log.info(`updating existing PR comment ${existing.id}`);
			await $({
				stdio: "inherit",
				env: process.env as Record<string, string>,
			})`gh api repos/${repo}/issues/comments/${String(existing.id)} -X PATCH -f ${`body=${body}`}`;
		} else {
			log.info("creating new PR comment");
			await $({
				stdio: "inherit",
				env: process.env as Record<string, string>,
			})`gh api repos/${repo}/issues/${String(prNumber)}/comments -f ${`body=${body}`}`;
		}
	});

program.parseAsync(process.argv).catch((err) => {
	log.error(String(err?.stack ?? err));
	process.exit(1);
});
