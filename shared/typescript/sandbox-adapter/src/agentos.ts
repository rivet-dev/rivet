import { posix } from "node:path";
import type {
	Sandbox,
	SandboxActorContext,
	SandboxAdapter,
	SandboxBinding,
	SandboxConnectOptions,
	SandboxExecOptions,
	SandboxExecResult,
	SandboxFileStat,
	SandboxOutputPage,
	SandboxProcess,
	SandboxProcessExit,
	SandboxSpawnOptions,
} from "./types.js";

const DEFAULT_CWD = "/workspace";
const DEFAULT_MAX_OUTPUT_BYTES = 4 * 1024 * 1024;
const OUTPUT_POLL_MS = 25;

export interface AgentOSActorHandle {
	process: {
		spawn(
			command: string,
			args?: string[],
			options?: {
				cwd?: string;
				env?: Record<string, string>;
				stdin?: ActorData;
				timeoutMs?: number;
				output?: { retainEvents?: boolean };
			},
		): Promise<{ pid: number }>;
		wait(pid: number): Promise<AgentOSProcessExit>;
		kill(pid: number): Promise<void>;
		signal(pid: number, signal: string): Promise<void>;
		writeStdin(pid: number, data: string | Uint8Array): Promise<void>;
		closeStdin(pid: number): Promise<void>;
		readOutput(
			pid: number,
			options?: { after?: number },
		): Promise<AgentOSOutputReplay>;
	};
	filesystem: {
		readFile(path: string): Promise<Uint8Array>;
		writeFile(path: string, content: string | Uint8Array): Promise<void>;
		stat(path: string): Promise<AgentOSStat>;
		readdir(path: string): Promise<string[]>;
		exists(path: string): Promise<boolean>;
		mkdir(path: string, options?: { recursive?: boolean }): Promise<void>;
		remove(path: string, options?: { recursive?: boolean }): Promise<void>;
	};
	destroy?(): Promise<void>;
}

interface AgentOSActorAccessor {
	getOrCreate(
		key: string[],
		options?: { params?: unknown },
	): AgentOSActorHandle;
}

type ActorData =
	| { encoding: "utf8"; data: string }
	| { encoding: "base64"; data: string };

interface AgentOSProcessExit {
	outcome: "exited" | "signalled" | "timed_out";
	exitCode?: number;
	signal?: string;
}

interface AgentOSOutputReplay {
	events: Array<{
		sequence: number;
		channel: "stdout" | "stderr" | "pty";
		chunk: Uint8Array | { encoding: "base64"; data: string };
		timestampMs?: number;
	}>;
	nextCursor: string;
	hasMore: boolean;
	truncated: boolean;
}

interface AgentOSStat {
	type?: "file" | "directory" | "symlink";
	isDirectory?: boolean;
	isSymbolicLink?: boolean;
	size?: number;
	mode?: number;
	mtimeMs?: number;
}

export interface AgentOSSandboxOptions<
	TContext extends SandboxActorContext = SandboxActorContext,
> {
	/** Registry key of an actor created with `agentOS()`. */
	actor: string;
	/** Working directory inside agentOS. Defaults to `/workspace`. */
	cwd?: string;
	/** Connection params forwarded to the agentOS actor. */
	params?: unknown | ((context: TContext) => unknown | Promise<unknown>);
	/** Override the deterministic agentOS actor key. */
	key?: (
		context: TContext,
		options: SandboxConnectOptions,
	) => readonly string[];
}

export class AgentOSSandboxConfigurationError extends Error {
	readonly code = "agentos_sandbox_configuration";

	constructor(message: string, options?: ErrorOptions) {
		super(message, options);
		this.name = "AgentOSSandboxConfigurationError";
	}
}

/** Connects a Pi or other Rivet Actor to a separate `agentOS()` actor. */
export function agentOSSandbox<
	TContext extends SandboxActorContext = SandboxActorContext,
>(options: AgentOSSandboxOptions<TContext>): SandboxAdapter<TContext> {
	if (!options || typeof options.actor !== "string" || !options.actor.trim()) {
		throw new TypeError("agentOSSandbox requires an agentOS actor name");
	}
	const actor = options.actor.trim();
	const configuredCwd = normalizeCwd(options.cwd ?? DEFAULT_CWD);

	const resolveHandle = async (
		context: TContext,
		connectOptions: SandboxConnectOptions,
	): Promise<{ handle: AgentOSActorHandle; key: string[] }> => {
		try {
			const client = context.client() as Record<string, unknown>;
			const accessor = client[actor] as AgentOSActorAccessor | undefined;
			if (!accessor || typeof accessor.getOrCreate !== "function") {
				throw new Error(`registry has no actor named ${JSON.stringify(actor)}`);
			}
			const key = resolveActorKey(options, context, connectOptions);
			const params =
				typeof options.params === "function"
					? await options.params(context)
					: options.params;
			const handle = accessor.getOrCreate(key, { params });
			assertAgentOSHandle(handle);
			return { handle, key };
		} catch (cause) {
			if (cause instanceof AgentOSSandboxConfigurationError) throw cause;
			throw new AgentOSSandboxConfigurationError(
				`agentOS actor ${JSON.stringify(actor)} could not be used: ${cause instanceof Error ? cause.message : String(cause)}`,
				{ cause },
			);
		}
	};

	return {
		async connect(context, connectOptions) {
			const { handle, key } = await resolveHandle(context, connectOptions);
			const cwd = normalizeCwd(connectOptions.cwd ?? configuredCwd);
			await handle.filesystem.mkdir(cwd, { recursive: true });
			return createAgentOSSandbox(handle, cwd, {
				provider: "agentos",
				actor,
				key,
			});
		},
		async destroy(context, lifecycleOptions) {
			const { handle } = await resolveHandle(context, lifecycleOptions);
			if (typeof handle.destroy !== "function") {
				throw new AgentOSSandboxConfigurationError(
					`agentOS actor ${JSON.stringify(actor)} does not expose actor destruction`,
				);
			}
			await handle.destroy();
		},
	};
}

function resolveActorKey<TContext extends SandboxActorContext>(
	options: AgentOSSandboxOptions<TContext>,
	context: TContext,
	connectOptions: SandboxConnectOptions,
): string[] {
	const persisted = bindingActorKey(connectOptions.binding, options.actor);
	const raw =
		persisted ??
		options.key?.(context, connectOptions) ??
		["sandbox", context.actorId, connectOptions.id];
	const key = [...raw];
	if (key.length === 0 || key.some((part) => !part)) {
		throw new AgentOSSandboxConfigurationError(
			"agentOS sandbox actor keys must contain non-empty strings",
		);
	}
	return key;
}

function bindingActorKey(
	binding: SandboxBinding | undefined,
	actor: string,
): string[] | undefined {
	if (binding === undefined) {
		return undefined;
	}
	if (binding === null || typeof binding !== "object" || Array.isArray(binding)) {
		throw new AgentOSSandboxConfigurationError(
			"persisted agentOS sandbox binding must be an object",
		);
	}
	if (binding.provider !== "agentos" || binding.actor !== actor) {
		throw new AgentOSSandboxConfigurationError(
			"persisted sandbox binding belongs to a different provider or agentOS actor",
		);
	}
	if (!Array.isArray(binding.key) || !binding.key.every((part) => typeof part === "string")) {
		throw new AgentOSSandboxConfigurationError(
			"persisted agentOS sandbox binding has an invalid actor key",
		);
	}
	return binding.key;
}

function createAgentOSSandbox(
	handle: AgentOSActorHandle,
	cwd: string,
	binding: SandboxBinding,
): Sandbox {
	return {
		binding,
		cwd,
		async exec(command, options) {
			const process = await spawnAgentOSProcess(
				handle,
				"sh",
				["-lc", command],
				{
					...options,
					cwd: options?.cwd ?? cwd,
					retainOutput: true,
				},
			);
			return collectProcessOutput(process, options);
		},
		async spawn(command, args = [], options) {
			return spawnAgentOSProcess(handle, command, [...args], {
				...options,
				cwd: options?.cwd ?? cwd,
			});
		},
		async readFile(path, options) {
			if (options?.maxBytes !== undefined) {
				const stat = normalizeStat(await handle.filesystem.stat(path));
				if (stat.size > options.maxBytes) {
					throw new Error(
						`Sandbox file is ${stat.size} bytes; limit is ${options.maxBytes} bytes`,
					);
				}
			}
			const data = await handle.filesystem.readFile(path);
			if (options?.maxBytes !== undefined && data.byteLength > options.maxBytes) {
				throw new Error(
					`Sandbox file exceeded the ${options.maxBytes} byte read limit`,
				);
			}
			return data;
		},
		writeFile: (path, content) => handle.filesystem.writeFile(path, content),
		async stat(path) {
			return normalizeStat(await handle.filesystem.stat(path));
		},
		readdir: (path) => handle.filesystem.readdir(path),
		exists: (path) => handle.filesystem.exists(path),
		mkdir: (path, mkdirOptions) =>
			handle.filesystem.mkdir(path, mkdirOptions),
		async remove(path, removeOptions) {
			if (removeOptions?.force && !(await handle.filesystem.exists(path))) return;
			try {
				await handle.filesystem.remove(path, {
					recursive: removeOptions?.recursive,
				});
			} catch (error) {
				if (removeOptions?.force && !(await handle.filesystem.exists(path))) return;
				throw error;
			}
		},
	};
}

async function spawnAgentOSProcess(
	handle: AgentOSActorHandle,
	command: string,
	args: string[],
	options: SandboxSpawnOptions | undefined,
): Promise<SandboxProcess> {
	throwIfAborted(options?.signal);
	let process: SandboxProcess | undefined;
	let abortRequested = false;
	const abort = () => {
		abortRequested = true;
		if (process) void process.kill("SIGTERM").catch(() => {});
	};
	options?.signal?.addEventListener("abort", abort, { once: true });
	try {
		const descriptor = await handle.process.spawn(command, args, {
			cwd: options?.cwd,
			env: options?.env,
			stdin: encodeActorData(options?.stdin),
			timeoutMs: options?.timeoutMs,
			output: { retainEvents: options?.retainOutput ?? true },
		});
		process = createAgentOSProcess(handle, descriptor.pid, () =>
			options?.signal?.removeEventListener("abort", abort),
		);
		if (abortRequested || options?.signal?.aborted) {
			await process.kill("SIGTERM").catch(() => {});
			throw options?.signal?.reason ?? new Error("sandbox operation aborted");
		}
		return process;
	} catch (error) {
		options?.signal?.removeEventListener("abort", abort);
		throw error;
	}
}

function createAgentOSProcess(
	handle: AgentOSActorHandle,
	pid: number,
	dispose: () => void,
): SandboxProcess {
	return {
		pid,
		async readOutput(options): Promise<SandboxOutputPage> {
			const replay = await handle.process.readOutput(pid, {
				after: options?.after,
			});
			const source = replay.events.slice(0, options?.maxEvents);
			let bytes = 0;
			let pageLimited = false;
			const events = source.flatMap((event) => {
				const decoded = decodeChunk(event.chunk);
				const remaining = (options?.maxBytes ?? Number.POSITIVE_INFINITY) - bytes;
				if (remaining <= 0) {
					pageLimited = true;
					return [];
				}
				const data = decoded.subarray(0, remaining);
				bytes += data.byteLength;
				if (data.byteLength < decoded.byteLength) pageLimited = true;
				return [{
					sequence: event.sequence,
					stream: event.channel,
					data,
					timestampMs: event.timestampMs,
				}];
			});
			return {
				events,
				nextSequence: events.at(-1)?.sequence ?? options?.after ?? -1,
				hasMore: replay.hasMore || source.length < replay.events.length,
				truncated: replay.truncated || pageLimited,
			};
		},
		async wait(): Promise<SandboxProcessExit> {
			try {
				const result = await handle.process.wait(pid);
				return {
					exitCode: result.exitCode ?? null,
					outcome: result.outcome,
					signal: result.signal,
				};
			} finally {
				dispose();
			}
		},
		writeStdin: (data) => handle.process.writeStdin(pid, data),
		closeStdin: () => handle.process.closeStdin(pid),
		async kill(signal) {
			dispose();
			return signal
				? handle.process.signal(pid, signal)
				: handle.process.kill(pid);
		},
	};
}

async function collectProcessOutput(
	process: SandboxProcess,
	options: SandboxExecOptions | undefined,
): Promise<SandboxExecResult> {
	const limit = options?.maxOutputBytes ?? DEFAULT_MAX_OUTPUT_BYTES;
	if (!Number.isSafeInteger(limit) || limit <= 0) {
		throw new TypeError("maxOutputBytes must be a positive safe integer");
	}
	let after = -1;
	let bytes = 0;
	let stdout = "";
	let stderr = "";
	let truncated = false;
	let settled = false;
	let exit: SandboxProcessExit | undefined;
	let waitError: unknown;
	const wait = process.wait().then(
		(result) => {
			exit = result;
			settled = true;
		},
		(error) => {
			waitError = error;
			settled = true;
		},
	);

	while (true) {
		const page = await process.readOutput({ after, maxBytes: limit - bytes });
		for (const event of page.events) {
			after = Math.max(after, event.sequence);
			bytes += event.data.byteLength;
			options?.onOutput?.(event);
			const text = Buffer.from(event.data).toString();
			if (event.stream === "stderr") stderr += text;
			else stdout += text;
		}
		truncated ||= page.truncated;
		if (bytes >= limit) {
			truncated = true;
			await process.kill("SIGTERM").catch(() => {});
		}
		if (!settled && !truncated && !page.hasMore) {
			await Promise.race([wait, delay(OUTPUT_POLL_MS)]);
		}
		if (truncated || (settled && !page.hasMore)) break;
	}
	await wait;
	if (waitError) throw waitError;
	throwIfAborted(options?.signal);
	return {
		exitCode: exit?.exitCode ?? null,
		stdout,
		stderr,
		outcome: exit?.outcome,
		signal: exit?.signal,
		truncated,
	};
}

function delay(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

function normalizeStat(stat: AgentOSStat): SandboxFileStat {
	const type = stat.type
		? stat.type
		: stat.isSymbolicLink
			? "symlink"
			: stat.isDirectory
				? "directory"
				: "file";
	return {
		type,
		size: stat.size ?? 0,
		mode: stat.mode,
		mtimeMs: stat.mtimeMs,
	};
}

function encodeActorData(
	value: string | Uint8Array | undefined,
): ActorData | undefined {
	if (value === undefined) return undefined;
	return typeof value === "string"
		? { encoding: "utf8", data: value }
		: { encoding: "base64", data: Buffer.from(value).toString("base64") };
}

function decodeChunk(
	chunk: Uint8Array | { encoding: "base64"; data: string },
): Uint8Array {
	return chunk instanceof Uint8Array
		? chunk
		: new Uint8Array(Buffer.from(chunk.data, "base64"));
}

function normalizeCwd(cwd: string): string {
	if (!posix.isAbsolute(cwd)) {
		throw new TypeError("sandbox cwd must be an absolute POSIX path");
	}
	return posix.normalize(cwd);
}

function throwIfAborted(signal: AbortSignal | undefined): void {
	if (signal?.aborted) {
		throw signal.reason ?? new Error("sandbox operation aborted");
	}
}

function assertAgentOSHandle(
	handle: AgentOSActorHandle,
): asserts handle is AgentOSActorHandle {
	const candidate = handle as unknown as Record<string, unknown>;
	const process = candidate.process as Record<string, unknown> | undefined;
	const filesystem = candidate.filesystem as Record<string, unknown> | undefined;
	for (const method of [
		"spawn",
		"wait",
		"kill",
		"signal",
		"writeStdin",
		"closeStdin",
		"readOutput",
	]) {
		if (typeof process?.[method] !== "function") {
			throw new Error(
				`selected actor is not an @rivet-dev/agentos actor (missing process.${method})`,
			);
		}
	}
	for (const method of [
		"readFile",
		"writeFile",
		"stat",
		"readdir",
		"exists",
		"mkdir",
		"remove",
	]) {
		if (typeof filesystem?.[method] !== "function") {
			throw new Error(
				`selected actor is not an @rivet-dev/agentos actor (missing filesystem.${method})`,
			);
		}
	}
}
