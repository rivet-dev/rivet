#!/usr/bin/env node
import { existsSync, readFileSync, statSync } from "node:fs";
import { isAbsolute, relative, resolve } from "node:path";
import { spawnSync } from "node:child_process";

const invocationCwd = process.cwd();
const root = jj(["workspace", "root"]) || git(["rev-parse", "--show-toplevel"]) || invocationCwd;
process.chdir(root);

const stagedOnly = process.argv.includes("--staged");
const rustOnly = process.argv.includes("--rust-only");
const hookFiles = await filesFromHookInput();
const files = hookFiles !== null ? hookFiles : filesFromGitChanges(stagedOnly);

const existingFiles = unique(files)
	.map((path) => normalizeRepoPath(path))
	.filter((path) => path && existsSync(path) && statSync(path).isFile())
	.filter((path) => !isGeneratedOutput(path));

const biomeFiles = existingFiles.filter(isBiomeFile);
const rustFiles = existingFiles.filter((path) => path.endsWith(".rs"));

if (!rustOnly && biomeFiles.length > 0) {
	run("pnpm", [
		"biome",
		"format",
		"--write",
		"--files-ignore-unknown=true",
		"--no-errors-on-unmatched",
		...biomeFiles,
	]);
}

if (rustFiles.length > 0) {
	run("rustfmt", ["--config", "skip_children=true", ...rustFiles]);
}

async function filesFromHookInput() {
	if (process.stdin.isTTY) return null;

	const raw = readFileSync(0, "utf8");
	if (!raw.trim()) return null;

	const files = [];
	let inputCwd = invocationCwd;
	try {
		const payload = JSON.parse(raw);
		if (typeof payload.cwd === "string") inputCwd = payload.cwd;
		collectPathValues(payload.tool_input, files);
		if (typeof payload.tool_input?.command === "string") {
			collectPatchPaths(payload.tool_input.command, files);
		}
	} catch {
		collectPatchPaths(raw, files);
	}

	return files.map((path) => (isAbsolute(path) ? path : resolve(inputCwd, path)));
}

function collectPatchPaths(patch, files) {
	for (const match of patch.matchAll(/^\*\*\* (?:Add|Update|Delete) File: (.+)$/gm)) {
		files.push(match[1].trim());
	}
	for (const match of patch.matchAll(/^\*\*\* Move to: (.+)$/gm)) {
		files.push(match[1].trim());
	}
}

function collectPathValues(value, files) {
	if (!value || typeof value !== "object") return;

	for (const [key, nested] of Object.entries(value)) {
		if (
			typeof nested === "string" &&
			(key === "file_path" || key === "path" || key === "filename")
		) {
			files.push(nested);
			continue;
		}

		collectPathValues(nested, files);
	}
}

function filesFromGitChanges(stagedOnly) {
	if (stagedOnly) {
		return gitLines(["diff", "--name-only", "--cached", "--diff-filter=ACMR"]);
	}

	return unique([
		...gitLines(["diff", "--name-only", "--diff-filter=ACMR"]),
		...gitLines(["diff", "--name-only", "--cached", "--diff-filter=ACMR"]),
		...gitLines(["ls-files", "--others", "--exclude-standard"]),
	]);
}

function normalizeRepoPath(path) {
	const absolute = isAbsolute(path) ? path : resolve(root, path);
	const relativePath = relative(root, absolute);
	if (!relativePath || relativePath.startsWith("..")) return null;
	return relativePath;
}

function isGeneratedOutput(path) {
	return (
		/^engine\/sdks\/[^/]+\/api-[^/]+\//.test(path) ||
		/^engine\/sdks\/typescript\/envoy-protocol\//.test(path) ||
		/^rivetkit-typescript\/packages\/rivetkit\/src\/common\/bare\/generated\//.test(
			path,
		) ||
		path === "rivetkit-typescript/packages/rivetkit-napi/index.d.ts" ||
		path === "rivetkit-typescript/packages/rivetkit-wasm/index.d.ts"
	);
}

function isBiomeFile(path) {
	return /\.(?:ts|tsx|js|jsx|mjs|cjs|css|json|jsonc)$/.test(path);
}

function unique(values) {
	return [...new Set(values.filter(Boolean))];
}

function git(args) {
	const result = spawnSync("git", args, { encoding: "utf8" });
	return result.status === 0 ? result.stdout.trim() : null;
}

function jj(args) {
	const result = spawnSync("jj", args, { encoding: "utf8" });
	return result.status === 0 ? result.stdout.trim() : null;
}

function gitLines(args) {
	return (git(args) || "").split("\n").filter(Boolean);
}

function run(command, args) {
	const result = spawnSync(command, args, { stdio: "inherit" });
	if (result.status !== 0) process.exit(result.status ?? 1);
}
