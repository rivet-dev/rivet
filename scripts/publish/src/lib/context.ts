import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { $ } from "execa";

/**
 * Publish context. Resolved once per workflow run by the `context-output` CI
 * subcommand and passed through every subsequent step via GitHub Actions job
 * outputs + per-step flags.
 */
export type Trigger = "pr" | "main" | "release";

export interface PublishContext {
	trigger: Trigger;
	/** Resolved version string, never null. */
	version: string;
	/** npm dist-tag. */
	npmTag: string;
	/** Short commit sha (7 chars). */
	sha: string;
	/** Only meaningful when trigger === "release". */
	latest: boolean;
	prNumber?: number;
	repoRoot: string;
}

/** Override set accepted by the local release cutter. */
export interface ResolveOverrides {
	trigger?: Trigger;
	version?: string;
	latest?: boolean;
	prNumber?: number;
	sha?: string;
}

function findRepoRoot(): string {
	if (process.env.GITHUB_WORKSPACE && existsSync(process.env.GITHUB_WORKSPACE)) {
		return process.env.GITHUB_WORKSPACE;
	}
	let dir = dirname(fileURLToPath(import.meta.url));
	for (let i = 0; i < 10; i++) {
		if (existsSync(join(dir, "pnpm-workspace.yaml"))) return dir;
		dir = dirname(dir);
	}
	throw new Error("Could not locate repo root (no pnpm-workspace.yaml)");
}

/**
 * Source of truth for the base version used when computing preview
 * pre-release strings. Deliberately read from `rivetkit-native`'s committed
 * `package.json`. The committed value is expected to be a plain semver (e.g.
 * `2.5.0`), not a bumped preview version — `bumpPackageJsons` writes
 * previews to disk but CI runs it after context resolution, so the read
 * always sees the pristine committed value.
 */
function readBaseVersion(repoRoot: string): string {
	const pkgPath = join(
		repoRoot,
		"rivetkit-typescript/packages/rivetkit-native/package.json",
	);
	const pkg = JSON.parse(readFileSync(pkgPath, "utf-8")) as { version: string };
	// Strip any trailing prerelease so previews keyed off an rc base still
	// compose cleanly as `{base-without-prerelease}-pr.N.sha`.
	const idx = pkg.version.indexOf("-");
	return idx === -1 ? pkg.version : pkg.version.slice(0, idx);
}

async function readShortSha(repoRoot: string): Promise<string> {
	const envSha = process.env.GITHUB_SHA;
	if (envSha) return envSha.slice(0, 7);
	const { stdout } = await $({ cwd: repoRoot })`git rev-parse HEAD`;
	return stdout.trim().slice(0, 7);
}

function readPrNumberFromEvent(): number | undefined {
	const path = process.env.GITHUB_EVENT_PATH;
	if (!path || !existsSync(path)) return undefined;
	try {
		const event = JSON.parse(readFileSync(path, "utf-8")) as {
			pull_request?: { number?: number };
			number?: number;
		};
		if (typeof event.pull_request?.number === "number") {
			return event.pull_request.number;
		}
		if (typeof event.number === "number") return event.number;
	} catch {
		// fall through
	}
	return undefined;
}

function readInputFromEvent<T = unknown>(name: string): T | undefined {
	const path = process.env.GITHUB_EVENT_PATH;
	if (!path || !existsSync(path)) return undefined;
	try {
		const event = JSON.parse(readFileSync(path, "utf-8")) as {
			inputs?: Record<string, unknown>;
		};
		const v = event.inputs?.[name];
		return v as T | undefined;
	} catch {
		return undefined;
	}
}

function parseBoolInput(v: unknown, fallback: boolean): boolean {
	if (typeof v === "boolean") return v;
	if (typeof v === "string") {
		if (v === "true") return true;
		if (v === "false") return false;
	}
	return fallback;
}

function deriveTrigger(overrides: ResolveOverrides | undefined): Trigger {
	if (overrides?.trigger) return overrides.trigger;
	const eventName = process.env.GITHUB_EVENT_NAME;
	if (eventName === "pull_request" || eventName === "pull_request_target")
		return "pr";
	if (eventName === "workflow_dispatch") return "release";
	if (eventName === "push") return "main";
	// Default for local invocation without overrides (unusual): assume release
	// so missing fields are caught loudly.
	return "release";
}

function computeNpmTag(
	trigger: Trigger,
	version: string,
	latest: boolean,
	prNumber?: number,
): string {
	if (trigger === "pr") {
		if (typeof prNumber !== "number") {
			throw new Error("PR trigger requires prNumber to compute npm tag");
		}
		return `pr-${prNumber}`;
	}
	if (trigger === "main") return "main";
	// release
	if (version.includes("-rc.")) return "rc";
	return latest ? "latest" : "next";
}

function computeVersion(
	trigger: Trigger,
	base: string,
	sha: string,
	prNumber: number | undefined,
	overrideVersion: string | undefined,
): string {
	if (overrideVersion) return overrideVersion;
	if (trigger === "pr") {
		if (typeof prNumber !== "number") {
			throw new Error("PR trigger requires prNumber to compute version");
		}
		return `${base}-pr.${prNumber}.${sha}`;
	}
	if (trigger === "main") return `${base}-main.${sha}`;
	throw new Error("release trigger requires an explicit version override");
}

/**
 * Resolve the publish context. Pure function of environment + overrides.
 * Not memoized: each subcommand process re-reads env, and the `context-output`
 * subcommand exists specifically so downstream steps receive stable values via
 * `$GITHUB_OUTPUT` / flags instead of re-resolving.
 */
export async function resolveContext(
	overrides: ResolveOverrides = {},
): Promise<PublishContext> {
	const repoRoot = findRepoRoot();
	const trigger = deriveTrigger(overrides);

	const sha = overrides.sha ?? (await readShortSha(repoRoot));

	let prNumber = overrides.prNumber;
	if (trigger === "pr" && prNumber === undefined) {
		prNumber = readPrNumberFromEvent();
	}

	// Release version: override > workflow_dispatch input > error.
	let version = overrides.version;
	if (!version && trigger === "release") {
		const input = readInputFromEvent<string>("version");
		if (typeof input === "string" && input.length > 0) version = input;
	}

	if (trigger !== "release") {
		version = computeVersion(
			trigger,
			readBaseVersion(repoRoot),
			sha,
			prNumber,
			version,
		);
	} else if (!version) {
		throw new Error(
			"release trigger requires version (pass --version or workflow_dispatch input)",
		);
	}

	// Latest: override > workflow_dispatch input > false.
	let latest = overrides.latest;
	if (latest === undefined) {
		const input = readInputFromEvent<unknown>("latest");
		latest = parseBoolInput(input, false);
	}
	if (trigger !== "release") latest = false;

	const npmTag = computeNpmTag(trigger, version, latest, prNumber);

	return {
		trigger,
		version,
		npmTag,
		sha,
		latest,
		prNumber,
		repoRoot,
	};
}

/** Write every context field to `$GITHUB_OUTPUT` so downstream steps read via needs.*. */
export function writeContextToGithubOutput(ctx: PublishContext): void {
	const path = process.env.GITHUB_OUTPUT;
	if (!path) {
		// When invoked locally for debugging, print to stdout in the same format.
		console.log(`trigger=${ctx.trigger}`);
		console.log(`version=${ctx.version}`);
		console.log(`npm_tag=${ctx.npmTag}`);
		console.log(`sha=${ctx.sha}`);
		console.log(`latest=${ctx.latest}`);
		if (ctx.prNumber !== undefined) console.log(`pr_number=${ctx.prNumber}`);
		return;
	}
	const lines = [
		`trigger=${ctx.trigger}`,
		`version=${ctx.version}`,
		`npm_tag=${ctx.npmTag}`,
		`sha=${ctx.sha}`,
		`latest=${ctx.latest}`,
	];
	if (ctx.prNumber !== undefined) lines.push(`pr_number=${ctx.prNumber}`);
	// Append (do not overwrite) in case other steps also wrote to GITHUB_OUTPUT.
	const { appendFileSync } = require("node:fs") as typeof import("node:fs");
	appendFileSync(path, `${lines.join("\n")}\n`);
}
