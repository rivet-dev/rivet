import { posix } from "node:path";
import {
	createBashToolDefinition,
	createEditToolDefinition,
	createFindToolDefinition,
	createGrepToolDefinition,
	createLsToolDefinition,
	createReadToolDefinition,
	createWriteToolDefinition,
	type BashOperations,
	type GrepToolDetails,
	type GrepToolInput,
	type ToolDefinition,
} from "@earendil-works/pi-coding-agent";
import type { Sandbox } from "@rivet-dev/sandbox-adapter";

const MAX_SEARCH_RESULTS = 1_000;
const MAX_SEARCH_BYTES = 512 * 1024;
const MAX_FILE_BYTES = 8 * 1024 * 1024;

/** Creates Pi's seven built-in coding tools backed entirely by a sandbox. */
export function createSandboxTools(
	sandbox: Sandbox,
): ToolDefinition<any, any, any>[] {
	const cwd = normalizeRoot(sandbox.cwd);
	const resolvePath = (path: string) => resolveSandboxPath(cwd, path);

	const read = createReadToolDefinition(cwd, {
		operations: {
			async access(path) {
				const resolved = resolvePath(path);
				if (!(await sandbox.exists(resolved))) {
					throw new Error(`Path not found: ${resolved}`);
				}
			},
			async readFile(path) {
				return Buffer.from(
					await sandbox.readFile(resolvePath(path), {
						maxBytes: MAX_FILE_BYTES,
					}),
				);
			},
			detectImageMimeType: async (path) => detectImageMimeType(path),
		},
	});
	const write = createWriteToolDefinition(cwd, {
		operations: {
			mkdir: (path) => sandbox.mkdir(resolvePath(path), { recursive: true }),
			writeFile: (path, content) =>
				sandbox.writeFile(resolvePath(path), content),
		},
	});
	const edit = createEditToolDefinition(cwd, {
		operations: {
			async access(path) {
				const resolved = resolvePath(path);
				if (!(await sandbox.exists(resolved))) {
					throw new Error(`Path not found: ${resolved}`);
				}
			},
			async readFile(path) {
				return Buffer.from(
					await sandbox.readFile(resolvePath(path), {
						maxBytes: MAX_FILE_BYTES,
					}),
				);
			},
			writeFile: (path, content) =>
				sandbox.writeFile(resolvePath(path), content),
		},
	});
	const bash = createBashToolDefinition(cwd, {
		operations: createSandboxBashOperations(sandbox),
	});
	const ls = createLsToolDefinition(cwd, {
		operations: {
			exists: (path) => sandbox.exists(resolvePath(path)),
			async readdir(path) {
				return sandbox.readdir(resolvePath(path));
			},
			async stat(path) {
				const stat = await sandbox.stat(resolvePath(path));
				return { isDirectory: () => stat.type === "directory" };
			},
		},
	});
	const find = createFindToolDefinition(cwd, {
		operations: {
			exists: (path) => sandbox.exists(resolvePath(path)),
			async glob(pattern, requestedCwd, options) {
				const searchRoot = resolvePath(requestedCwd);
				const result = await sandbox.exec(
					"rg --files --null --hidden --glob '!**/.git/**' --glob '!**/node_modules/**'",
					{ cwd: searchRoot, maxOutputBytes: MAX_SEARCH_BYTES },
				);
				assertCommandSucceeded(
					"find",
					result.exitCode,
					result.stderr,
					result.truncated,
				);
				const matcher = globRegex(pattern);
				return result.stdout
					.split("\0")
					.map((path) => path.replace(/^\.\//, ""))
					.filter((path) => path.length > 0 && matcher.test(path))
					.slice(0, Math.min(options.limit, MAX_SEARCH_RESULTS));
			},
		},
	});

	const grepBase = createGrepToolDefinition(cwd);
	const grep = {
		...grepBase,
		execute: async (
			_toolCallId: string,
			params: GrepToolInput,
			signal: AbortSignal | undefined,
		) => {
			if (signal?.aborted) throw abortError(signal);
			const searchPath = resolvePath(params.path ?? ".");
			const limit = Math.min(
				Math.max(1, params.limit ?? 100),
				MAX_SEARCH_RESULTS,
			);
			const args = ["--line-number", "--color=never", "--hidden"];
			if (params.ignoreCase) args.push("--ignore-case");
			if (params.literal) args.push("--fixed-strings");
			if (params.glob) args.push("--glob", params.glob);
			if (params.context && params.context > 0) {
				args.push("--context", String(Math.floor(params.context)));
			}
			args.push("--glob", "!**/.git/**", "--glob", "!**/node_modules/**");
			args.push("--", params.pattern, searchPath);
			const result = await sandbox.exec(
				`rg ${args.map(shellQuote).join(" ")}`,
				{ cwd, signal, maxOutputBytes: MAX_SEARCH_BYTES },
			);
			if (signal?.aborted) throw abortError(signal);
			if (
				!result.truncated &&
				result.exitCode !== 0 &&
				result.exitCode !== 1
			) {
				throw new Error(result.stderr.trim() || `ripgrep exited with ${result.exitCode}`);
			}
			const allLines = result.stdout.replace(/\n$/, "").split("\n");
			const matchedLines = allLines[0] === "" ? [] : allLines;
			const selected = matchedLines.slice(0, limit);
			let output = selected.join("\n");
			let linesTruncated = result.truncated ?? false;
			if (Buffer.byteLength(output) > MAX_SEARCH_BYTES) {
				output = Buffer.from(output).subarray(0, MAX_SEARCH_BYTES).toString();
				linesTruncated = true;
			}
			const details: GrepToolDetails = {};
			if (matchedLines.length > selected.length) details.matchLimitReached = limit;
			if (linesTruncated) details.linesTruncated = true;
			return {
				content: [
					{
						type: "text",
						text: output || "No matches found",
					},
				],
				details: Object.keys(details).length > 0 ? details : undefined,
			};
		},
	} as unknown as ToolDefinition<any, GrepToolDetails | undefined, any>;

	return [read, bash, edit, write, grep, find, ls];
}

/** Creates the operations used by Pi's direct `executeBash` API. */
export function createSandboxBashOperations(sandbox: Sandbox): BashOperations {
	const cwd = normalizeRoot(sandbox.cwd);
	return {
		async exec(command, requestedCwd, options) {
			const result = await sandbox.exec(command, {
				cwd: resolveSandboxPath(cwd, requestedCwd),
				env: stringEnvironment(options.env),
				timeoutMs: options.timeout,
				signal: options.signal,
				onOutput: (event) => options.onData(Buffer.from(event.data)),
			});
			return { exitCode: result.exitCode };
		},
	};
}

export function resolveSandboxPath(root: string, path: string): string {
	const normalizedRoot = normalizeRoot(root);
	const resolved = posix.resolve(normalizedRoot, path);
	const prefix = normalizedRoot === "/" ? "/" : `${normalizedRoot}/`;
	if (resolved !== normalizedRoot && !resolved.startsWith(prefix)) {
		throw new Error(`Path escapes sandbox working directory: ${path}`);
	}
	return resolved;
}

function normalizeRoot(path: string): string {
	if (!posix.isAbsolute(path)) {
		throw new Error("sandbox cwd must be an absolute POSIX path");
	}
	return posix.normalize(path);
}

function stringEnvironment(
	env: NodeJS.ProcessEnv | undefined,
): Record<string, string> | undefined {
	if (!env) return undefined;
	return Object.fromEntries(
		Object.entries(env).filter(
			(entry): entry is [string, string] => entry[1] !== undefined,
		),
	);
}

function detectImageMimeType(path: string): string | undefined {
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

function globRegex(pattern: string): RegExp {
	let expression = "^";
	for (let index = 0; index < pattern.length; index++) {
		const character = pattern[index]!;
		if (character === "*") {
			if (pattern[index + 1] === "*") {
				if (pattern[index + 2] === "/") {
					expression += "(?:.*/)?";
					index += 2;
				} else {
					expression += ".*";
					index++;
				}
			} else {
				expression += "[^/]*";
			}
		} else if (character === "?") {
			expression += "[^/]";
		} else {
			expression += character.replace(/[|\\{}()[\]^$+?.]/g, "\\$&");
		}
	}
	return new RegExp(`${expression}$`);
}

function assertCommandSucceeded(
	command: string,
	exitCode: number | null,
	stderr: string,
	truncated = false,
): void {
	if (exitCode !== 0 && !truncated) {
		throw new Error(stderr.trim() || `${command} exited with ${exitCode}`);
	}
}

function abortError(signal: AbortSignal): Error {
	return signal.reason instanceof Error
		? signal.reason
		: new Error("Operation aborted");
}
