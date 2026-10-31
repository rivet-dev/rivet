#!/usr/bin/env tsx
/**
 * Linear release cutter — called by humans, never by CI.
 *
 * Run from an EE jj workspace whose `@` is empty and directly on main@origin.
 * EE is the source of truth; OSS receives the bump through the snapshot sync.
 *
 * Steps:
 *   1. Resolve target version (flags → semver bump → error)
 *   2. Auto-detect or confirm `latest` flag against the OSS release tags
 *   3. Require an empty working-copy revision on the latest EE main and a syncable OSS main
 *   4. Print release plan and confirm
 *   5. Update non-package.json source files (Cargo.toml, examples)
 *   6. Rewrite every publishable package.json (via discovery)
 *   7. Commit the bump to EE main and push it
 *   8. Sync EE main to OSS main (builds and checks the public tree)
 *   9. Trigger the OSS publish.yaml workflow
 *
 * Debugging: comment out any step. No `--only-steps`, no phases.
 */
import { existsSync } from "node:fs";
import * as readline from "node:readline";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { Command } from "commander";
import { $ } from "execa";
import { scoped } from "../lib/logger.js";
import {
	bumpCargoVersions,
	bumpExampleLockfile,
	bumpPackageJsons,
	getLatestGitVersion,
	listRecentVersions,
	releaseLatest,
	resolveVersion,
	shouldTagAsLatest,
	updateSourceFiles,
} from "../lib/version.js";
import { OSS_REPOSITORY, requireEeReleaseRepository } from "../lib/repository.js";

const log = scoped("release");

function findRepoRoot(): string {
	let dir = dirname(fileURLToPath(import.meta.url));
	for (let i = 0; i < 10; i++) {
		if (existsSync(join(dir, "pnpm-workspace.yaml"))) return dir;
		dir = dirname(dir);
	}
	throw new Error("could not locate repo root");
}

async function confirmPrompt(question: string): Promise<boolean> {
	const rl = readline.createInterface({
		input: process.stdin,
		output: process.stdout,
	});
	const answer = await new Promise<string>((resolve) => {
		rl.question(question, resolve);
	});
	rl.close();
	return answer.trim().toLowerCase() === "yes" || answer.trim().toLowerCase() === "y";
}

interface CliOpts {
	version?: string;
	major?: boolean;
	minor?: boolean;
	patch?: boolean;
	latest?: boolean;
	noLatest?: boolean;
	dryRun?: boolean;
	yes?: boolean;
	skipChecks?: boolean;
}

async function main() {
	const program = new Command();
	program
		.name("cut-release")
		.description("Cut a new Rivet release (local orchestrator)")
		.option("--version <version>", "Explicit version (e.g. 2.5.0)")
		.option("--major", "Bump major")
		.option("--minor", "Bump minor")
		.option("--patch", "Bump patch")
		.option("--latest", "Mark as latest dist-tag")
		.option("--no-latest", "Do not mark as latest")
		.option("--dry-run", "Report version edits and run checks without writing or publishing")
		.option("-y, --yes", "Skip interactive confirmation")
		.option("--skip-checks", "Skip the OSS snapshot build; OSS CI and publish.yaml still build it")
		.parse();

	const opts = program.opts<CliOpts>();
	const repoRoot = findRepoRoot();
	await requireEeReleaseRepository(repoRoot);

	// 1. Resolve version.
	const version = await resolveVersion({
		version: opts.version,
		major: opts.major,
		minor: opts.minor,
		patch: opts.patch,
	});
	if (opts.latest && !releaseLatest(version, true)) {
		throw new Error(`prerelease ${version} cannot be marked latest`);
	}

	// 2. Latest flag: explicit > auto > false.
	let latest: boolean;
	if (opts.latest === true) latest = true;
	else if (opts.noLatest === true || opts.latest === false) latest = false;
	else latest = opts.dryRun ? false : await shouldTagAsLatest(version);
	latest = releaseLatest(version, latest);

	// 3. Require an empty working-copy revision directly on the latest EE main, and check that
	// the OSS sync can run (for example, that no OSS-only commit is waiting to be pulled)
	// before the bump is pushed to EE.
	if (!opts.dryRun) {
		await requireReleaseBase(repoRoot);
		await $({ stdio: "inherit", cwd: repoRoot })`scripts/git/oss-sync.sh push --dry-run --skip-build`;
	}

	// 4. Print plan.
	const latestGit = opts.dryRun ? null : await getLatestGitVersion();
	const recent = opts.dryRun ? [] : await listRecentVersions(10);
	console.log("");
	console.log("Release plan");
	console.log(`  Version:  ${version}`);
	console.log(`  Latest:   ${latest}`);
	console.log("  Source:   EE main, published to OSS main by scripts/git/oss-sync.sh");
	console.log(`  Previous: ${latestGit ?? "(none)"}`);
	if (opts.dryRun) console.log("  Dry run:  no source writes / commit / push / workflow trigger");
	console.log("");
	if (recent.length > 0) {
		console.log("Recent versions:");
		for (const v of recent) {
			const marker = v === latestGit ? " (latest)" : "";
			console.log(`  - ${v}${marker}`);
		}
		console.log("");
	}

	if (!opts.yes && !opts.dryRun) {
		const ok = await confirmPrompt("Proceed with release? (yes/no): ");
		if (!ok) {
			log.info("release cancelled");
			process.exit(0);
		}
	}

	// 5. Update non-package.json source files.
	log.info("updating source files (Cargo.toml, examples)");
	await updateSourceFiles(repoRoot, version, { dryRun: opts.dryRun });
	// Bump the Cargo.toml workspace dependency pins (the `version = "=X"`
	// exact pins on internal crates). updateSourceFiles only rewrites the
	// [workspace.package] version, so without this the internal crate pins
	// stay on the previous version and the Rust/wasm build fails to resolve.
	await bumpCargoVersions(repoRoot, version, { dryRun: opts.dryRun });
	await bumpExampleLockfile(repoRoot, version, { dryRun: opts.dryRun });

	// 6. Rewrite package.json version fields via discovery. Uses versionOnly
	// mode so `workspace:*` dep specs are preserved — the lockfile depends on
	// them. CI runs the full publish-time bump (with dep rewriting +
	// optionalDependencies injection) after `pnpm install --frozen-lockfile`.
	log.info("rewriting package.json versions");
	await bumpPackageJsons(repoRoot, version, {
		versionOnly: true,
		dryRun: opts.dryRun,
	});

	if (opts.dryRun) {
		log.info("dry run complete; no source files written or release actions performed");
		return;
	}

	// 7. Commit the bump to EE main. Cargo.lock follows the new workspace version.
	log.info("committing version bump to EE main");
	await $({ stdio: "inherit", cwd: repoRoot })`cargo update --workspace --offline`;
	await $({ stdio: "inherit", cwd: repoRoot })`jj commit -m ${`chore(release): update version to ${version}`}`;
	await $({ stdio: "inherit", cwd: repoRoot })`jj bookmark set main -r @-`;
	await $({ stdio: "inherit", cwd: repoRoot })`jj git push --remote origin --bookmark main`;

	// 8. Publish the bump to OSS main. The sync builds and checks the public tree, which is
	// exactly what publish.yaml builds, then records the sync back on EE main.
	log.info("syncing EE main to OSS main");
	const syncArgs = opts.skipChecks ? ["all", "--skip-build"] : ["all"];
	await $({ stdio: "inherit", cwd: repoRoot })`scripts/git/oss-sync.sh ${syncArgs}`;

	// 9. Trigger the workflow on OSS main, which now ends with the synced bump.
	log.info("triggering publish.yaml workflow");
	const latestFlag = latest ? "true" : "false";
	await $({
		stdio: "inherit",
		cwd: repoRoot,
	})`gh workflow run publish.yaml -R ${OSS_REPOSITORY} -f version=${version} -f latest=${latestFlag} --ref main`;

	console.log("");
	console.log(
		`Workflow triggered: https://github.com/${OSS_REPOSITORY}/actions/workflows/publish.yaml`,
	);
}

/** Fails unless `@` is an empty revision directly on top of the freshly fetched EE main. */
async function requireReleaseBase(repoRoot: string): Promise<void> {
	await $({ cwd: repoRoot })`jj git fetch --remote origin --branch main`;
	const { stdout } = await $({
		cwd: repoRoot,
	})`jj log --no-graph -r ${"@ & empty() & children(main@origin)"} -T commit_id`;
	if (!stdout.trim()) {
		throw new Error(
			"release needs an empty working-copy revision directly on main@origin; run `jj new main@origin` first",
		);
	}
}

main().catch((err) => {
	log.error(String(err?.stack ?? err));
	process.exit(1);
});
