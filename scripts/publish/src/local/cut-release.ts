#!/usr/bin/env tsx
/**
 * Linear release cutter — called by humans, never by CI.
 *
 * Steps:
 *   1. Resolve target version (flags → semver bump → error)
 *   2. Auto-detect or confirm `latest` flag
 *   3. Validate git working tree is clean
 *   4. Print release plan and confirm
 *   5. Update non-package.json source files (Cargo.toml, examples, sqlite-native)
 *   6. Rewrite every publishable package.json (via discovery)
 *   7. Run fern gen
 *   8. Run local type-check fail-fast
 *   9. Commit + push
 *   10. Trigger the publish.yaml workflow
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
	bumpPackageJsons,
	getLatestGitVersion,
	listRecentVersions,
	resolveVersion,
	shouldTagAsLatest,
	updateSourceFiles,
} from "../lib/version.js";
import { validateClean } from "../lib/git.js";

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
		.option("--dry-run", "Do not commit/push/trigger (still mutates source files)")
		.option("-y, --yes", "Skip interactive confirmation")
		.option("--skip-checks", "Skip local type-check fail-fast")
		.parse();

	const opts = program.opts<CliOpts>();
	const repoRoot = findRepoRoot();

	// 1. Resolve version.
	const version = await resolveVersion({
		version: opts.version,
		major: opts.major,
		minor: opts.minor,
		patch: opts.patch,
	});

	// 2. Latest flag: explicit > auto > false.
	let latest: boolean;
	if (opts.latest === true) latest = true;
	else if (opts.noLatest === true || opts.latest === false) latest = false;
	else latest = await shouldTagAsLatest(version);

	// 3. Validate git clean.
	await validateClean();

	// 4. Print plan.
	const { stdout: branch } = await $`git rev-parse --abbrev-ref HEAD`;
	const latestGit = await getLatestGitVersion();
	const recent = await listRecentVersions(10);
	console.log("");
	console.log("Release plan");
	console.log(`  Version:  ${version}`);
	console.log(`  Latest:   ${latest}`);
	console.log(`  Branch:   ${branch.trim()}`);
	console.log(`  Previous: ${latestGit ?? "(none)"}`);
	if (opts.dryRun) console.log("  Dry run:  no git commit / push / workflow trigger");
	console.log("");
	if (recent.length > 0) {
		console.log("Recent versions:");
		for (const v of recent) {
			const marker = v === latestGit ? " (latest)" : "";
			console.log(`  - ${v}${marker}`);
		}
		console.log("");
	}

	if (!opts.yes) {
		const ok = await confirmPrompt("Proceed with release? (yes/no): ");
		if (!ok) {
			log.info("release cancelled");
			process.exit(0);
		}
	}

	// 5. Update non-package.json source files.
	log.info("updating source files (Cargo.toml, examples, sqlite-native)");
	await updateSourceFiles(repoRoot, version);

	// 6. Rewrite package.json version fields via discovery. Uses versionOnly
	// mode so `workspace:*` dep specs are preserved — the lockfile depends on
	// them. CI runs the full publish-time bump (with dep rewriting +
	// optionalDependencies injection) after `pnpm install --frozen-lockfile`.
	log.info("rewriting package.json versions");
	await bumpPackageJsons(repoRoot, version, { versionOnly: true });

	// 7. Fern gen.
	log.info("running fern gen");
	await $({ stdio: "inherit", cwd: repoRoot })`./scripts/fern/gen.sh`;

	// 8. Local type-check fail-fast.
	if (!opts.skipChecks) {
		log.info("running local build + type-check (fail-fast)");
		await $({
			stdio: "inherit",
			cwd: repoRoot,
		})`pnpm build -F rivetkit -F @rivetkit/* -F !@rivetkit/shared-data -F !@rivetkit/engine-frontend -F !@rivetkit/mcp-hub -F !@rivetkit/sqlite-native -F !@rivetkit/sqlite-wasm -F !@rivetkit/rivetkit-native`;
		await $({
			stdio: "inherit",
			cwd: repoRoot,
		})`npx turbo build:pack-inspector -F rivetkit`;
		await $({ stdio: "inherit", cwd: repoRoot })`cargo check`;
	}

	if (opts.dryRun) {
		log.info("dry run complete — source files mutated, nothing committed");
		return;
	}

	// 9. Commit + push.
	log.info("committing version bump");
	await $({ stdio: "inherit", cwd: repoRoot })`git add .`;
	await $({
		stdio: "inherit",
		cwd: repoRoot,
		shell: true,
	})`git commit --allow-empty -m "chore(release): update version to ${version}"`;

	const currentBranch = (
		await $`git rev-parse --abbrev-ref HEAD`
	).stdout.trim();
	if (currentBranch === "main") {
		await $({ stdio: "inherit", cwd: repoRoot })`git push`;
	} else {
		// Use gt submit if available (Graphite), else fall back to git push.
		try {
			await $({
				stdio: "inherit",
				cwd: repoRoot,
			})`gt submit --force --no-edit --publish`;
		} catch {
			await $({
				stdio: "inherit",
				cwd: repoRoot,
			})`git push -u origin ${currentBranch}`;
		}
	}

	// 10. Trigger the workflow.
	log.info("triggering publish.yaml workflow");
	const latestFlag = latest ? "true" : "false";
	await $({
		stdio: "inherit",
		cwd: repoRoot,
	})`gh workflow run .github/workflows/publish.yaml -f version=${version} -f latest=${latestFlag} --ref ${currentBranch}`;

	const { stdout: repo } =
		await $`gh repo view --json nameWithOwner -q .nameWithOwner`;
	console.log("");
	console.log(
		`Workflow triggered: https://github.com/${repo.trim()}/actions/workflows/publish.yaml`,
	);
}

main().catch((err) => {
	log.error(String(err?.stack ?? err));
	process.exit(1);
});
