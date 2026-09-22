import { posix } from "node:path";
import {
	type BashOperations,
	createBashToolDefinition,
	createEditToolDefinition,
	createFindToolDefinition,
	createGrepToolDefinition,
	createLsToolDefinition,
	createReadToolDefinition,
	createWriteToolDefinition,
	type GrepToolDetails,
	type GrepToolInput,
	type ToolDefinition,
} from "@earendil-works/pi-coding-agent";
import type { Sandbox } from "@rivet-dev/sandbox-adapter";

const MAX_FIND_RESULTS = 1_000;
const MAX_GREP_MATCHES = 1_000;
const MAX_GREP_OUTPUT_BYTES = 512 * 1024;
const IGNORED_DIRECTORIES = [".git", "node_modules"];

/**
 * Builds Pi's seven built-in coding tools with every file and shell operation
 * routed into the sandbox. Passed as `customTools`, they replace the built-in
 * tools of the same names.
 */
export function createSandboxTools(
	sandbox: Sandbox,
): ToolDefinition<any, any, any>[] {
	const root = normalizeRoot(sandbox.cwd);
	const resolvePath = (path: string) => resolveSandboxPath(root, path);
	const access = async (path: string) => {
		const resolved = resolvePath(path);
		if (!(await sandbox.exists(resolved))) {
			throw new Error(`Path not found: ${resolved}`);
		}
	};
	const readFile = async (path: string) =>
		Buffer.from(await sandbox.readFile(resolvePath(path)));
	const writeFile = (path: string, content: string) =>
		sandbox.writeFile(resolvePath(path), content);

	const read = createReadToolDefinition(root, {
		operations: {
			access,
			readFile,
			detectImageMimeType: async (path) => imageMimeType(path),
		},
	});
	const write = createWriteToolDefinition(root, {
		operations: {
			writeFile,
			mkdir: (path) => sandbox.mkdir(resolvePath(path)),
		},
	});
	const edit = createEditToolDefinition(root, {
		operations: { access, readFile, writeFile },
	});
	const bash = createBashToolDefinition(root, {
		operations: createSandboxBashOperations(sandbox),
	});
	const ls = createLsToolDefinition(root, {
		operations: {
			exists: (path) => sandbox.exists(resolvePath(path)),
			readdir: (path) => sandbox.readdir(resolvePath(path)),
			stat: async (path) => {
				const stat = await sandbox.stat(resolvePath(path));
				return { isDirectory: () => stat.isDirectory };
			},
		},
	});
	const find = createFindToolDefinition(root, {
		operations: {
			exists: (path) => sandbox.exists(resolvePath(path)),
			glob: async (pattern, searchDirectory, options) => {
				const searchRoot = resolvePath(searchDirectory);
				const result = await sandbox.exec(
					`find . -type f ${IGNORED_DIRECTORIES.map((directory) => `-not -path ${shellQuote(`*/${directory}/*`)}`).join(" ")}`,
					{ cwd: searchRoot },
				);
				if (result.exitCode !== 0) {
					throw new Error(result.stderr.trim() || `find exited with ${result.exitCode}`);
				}
				return result.stdout
					.split("\n")
					.map(stripDotSlash)
					.filter((path) => path.length > 0 && matchesToolGlob(path, pattern))
					.slice(0, Math.min(options.limit, MAX_FIND_RESULTS));
			},
		},
	});

	const grepBase = createGrepToolDefinition(root);
	const grep = {
		...grepBase,
		execute: async (
			_toolCallId: string,
			params: GrepToolInput,
			signal: AbortSignal | undefined,
		) => {
			const searchPath = resolvePath(params.path ?? ".");
			const limit = Math.min(Math.max(1, params.limit ?? 100), MAX_GREP_MATCHES);
			const args = ["-r", "-n", "-I", "-H"];
			if (params.ignoreCase) args.push("-i");
			if (params.literal) args.push("-F");
			if (params.glob) args.push(`--include=${params.glob}`);
			if (params.context && params.context > 0) {
				args.push("-C", String(Math.floor(params.context)));
			}
			for (const directory of IGNORED_DIRECTORIES) {
				args.push(`--exclude-dir=${directory}`);
			}
			args.push("-e", params.pattern, "--", posix.relative(root, searchPath) || ".");
			const result = await sandbox.exec(`grep ${args.map(shellQuote).join(" ")}`, {
				cwd: root,
				signal,
			});
			if (result.exitCode !== 0 && result.exitCode !== 1) {
				throw new Error(result.stderr.trim() || `grep exited with ${result.exitCode}`);
			}
			const lines = result.stdout.replace(/\n$/, "").split("\n");
			const matched = lines[0] === "" ? [] : lines.map(stripDotSlash);
			const selected = matched.slice(0, limit);
			let output = selected.join("\n");
			let linesTruncated = false;
			if (Buffer.byteLength(output) > MAX_GREP_OUTPUT_BYTES) {
				output = Buffer.from(output).subarray(0, MAX_GREP_OUTPUT_BYTES).toString();
				linesTruncated = true;
			}
			const details: GrepToolDetails = {};
			if (matched.length > selected.length) details.matchLimitReached = limit;
			if (linesTruncated) details.linesTruncated = true;
			return {
				content: [{ type: "text", text: output || "No matches found" }],
				details: Object.keys(details).length > 0 ? details : undefined,
			};
		},
	} as unknown as ToolDefinition<any, GrepToolDetails | undefined, any>;

	return [read, bash, edit, write, grep, find, ls];
}

/**
 * Command execution for Pi's bash tool and `executeBash`, inside the sandbox.
 * Follows Pi's `BashOperations` contract: `timeout` is in seconds, and abort
 * and timeout reject with `aborted` and `timeout:<seconds>`.
 *
 * Pi passes the actor host's environment as `options.env`. It is not
 * forwarded, because it holds the host's provider keys; commands run with the
 * sandbox's own environment.
 */
export function createSandboxBashOperations(sandbox: Sandbox): BashOperations {
	const root = normalizeRoot(sandbox.cwd);
	return {
		async exec(command, requestedCwd, options) {
			const result = await sandbox.exec(command, {
				cwd: resolveSandboxPath(root, requestedCwd),
				timeoutMs: options.timeout === undefined ? undefined : options.timeout * 1000,
				signal: options.signal,
				onData: (chunk) => options.onData(Buffer.from(chunk)),
			});
			if (result.timedOut) {
				throw new Error(`timeout:${options.timeout}`);
			}
			return { exitCode: result.exitCode };
		},
	};
}

/** Resolves `path` against `root` and rejects anything outside it. */
export function resolveSandboxPath(root: string, path: string): string {
	const normalizedRoot = normalizeRoot(root);
	const resolved = posix.resolve(normalizedRoot, path);
	const prefix = normalizedRoot === "/" ? "/" : `${normalizedRoot}/`;
	if (resolved !== normalizedRoot && !resolved.startsWith(prefix)) {
		throw new Error(`Path escapes the sandbox working directory: ${path}`);
	}
	return resolved;
}

function normalizeRoot(path: string): string {
	if (!posix.isAbsolute(path)) {
		throw new Error(`sandbox cwd must be an absolute POSIX path, received ${path}`);
	}
	return posix.normalize(path);
}

/** Pi's find semantics: a pattern without a slash matches the file name at any depth. */
function matchesToolGlob(relativePath: string, pattern: string): boolean {
	if (pattern.includes("/")) {
		return (
			posix.matchesGlob(relativePath, pattern) ||
			posix.matchesGlob(relativePath, `**/${pattern}`)
		);
	}
	return posix.matchesGlob(posix.basename(relativePath), pattern);
}

function stripDotSlash(path: string): string {
	return path.startsWith("./") ? path.slice(2) : path;
}

function imageMimeType(path: string): string | undefined {
	const extension = posix.extname(path).toLowerCase();
	return {
		".bmp": "image/bmp",
		".gif": "image/gif",
		".jpeg": "image/jpeg",
		".jpg": "image/jpeg",
		".png": "image/png",
		".webp": "image/webp",
	}[extension];
}

function shellQuote(value: string): string {
	return `'${value.replaceAll("'", `'"'"'`)}'`;
}
