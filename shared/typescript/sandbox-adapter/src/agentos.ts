import { posix } from "node:path";
import type { Sandbox, SandboxActorContext, SandboxProvider } from "./index.js";
import {
	type RemoteOutputChunk,
	type RemoteProcessPoll,
	runRemoteProcess,
} from "./remote-process.js";

const DEFAULT_CWD = "/workspace";
/** Services runs the agentOS actor in this pool, locally and on Rivet Cloud. */
const SERVICES_POOL = "services";

type ProcessId = { generation: number | bigint; pid: number };

/**
 * The v1 agentOS actions this provider calls, typed structurally so this
 * package does not depend on `@rivet-dev/agentos`.
 */
export interface AgentOSActorHandle {
	v1: {
		process: {
			spawn(input: {
				command: string;
				args: string[];
				options: { cwd?: string; env?: Record<string, string> };
			}): Promise<ProcessId>;
			signal(input: { process: ProcessId; signal: "SIGKILL" }): Promise<void>;
			output: {
				read(input: { process: ProcessId; after?: number }): Promise<{
					events: Array<{
						sequence: number | bigint;
						stream: "stdout" | "stderr";
						data: Uint8Array;
					}>;
					hasMore: boolean;
					end?: { exitCode: number; signal?: string | null } | null;
				}>;
			};
		};
		filesystem: {
			readFile(input: { path: string }): Promise<Uint8Array>;
			writeFile(input: { path: string; content: string }): Promise<void>;
			mkdir(input: { path: string; recursive: boolean }): Promise<void>;
			stat(input: { path: string }): Promise<{ isDirectory: boolean }>;
			readdir(input: { path: string }): Promise<string[]>;
			exists(input: { path: string }): Promise<boolean>;
		};
	};
}

type AgentOSPackageSource = { url: string; digest?: string };

type AgentOSClient = {
	agentOS: {
		getOrCreate(
			key: string[],
			options: {
				poolName: string;
				createWithInput: {
					config: {
						filesystem: { root: { type: "durable" } };
						software?: AgentOSPackageSource[];
					};
				};
			},
		): AgentOSActorHandle;
	};
};

export interface AgentOSProviderOptions {
	/** Working directory inside the VM. Defaults to `/workspace`. */
	cwd?: string;
	/** Packages installed when the VM is created, for example coreutils for `sh`. */
	software?: AgentOSPackageSource[];
}

/**
 * Runs an agent's file and shell tools in the Services agentOS actor with the
 * key `["sandbox", <actor id>]`. The VM keeps its files in the actor's SQLite,
 * so they persist while the actor sleeps. The VM actor sleeps on its own when
 * idle, so the provider does not suspend it. Destroying the agent leaves the
 * VM actor and its files in place.
 */
export function agentOSProvider(options: AgentOSProviderOptions = {}): SandboxProvider {
	const cwd = posix.normalize(options.cwd ?? DEFAULT_CWD);
	if (!posix.isAbsolute(cwd)) {
		throw new Error(`agentOSProvider cwd must be an absolute path, received ${cwd}`);
	}
	const agentOS = (c: SandboxActorContext) => (c.client() as AgentOSClient).agentOS;

	return {
		name: "agentos",
		create: async (c) => c.actorId,
		connect: async (c, id) => {
			const handle = agentOS(c).getOrCreate(["sandbox", id], {
				poolName: SERVICES_POOL,
				createWithInput: {
					config: {
						filesystem: { root: { type: "durable" } },
						...(options.software ? { software: options.software } : {}),
					},
				},
			});
			await handle.v1.filesystem.mkdir({ path: cwd, recursive: true });
			return agentOSSandbox(handle, cwd);
		},
	};
}

function agentOSSandbox(handle: AgentOSActorHandle, cwd: string): Sandbox {
	const { process, filesystem } = handle.v1;
	return {
		cwd,
		exec: async (command, options) => {
			const id = await process.spawn({
				command: "sh",
				args: ["-c", command],
				options: { cwd: options.cwd, env: options.env },
			});
			return runRemoteProcess(
				{
					poll: (after) => readOutput(handle, id, after),
					kill: () => process.signal({ process: id, signal: "SIGKILL" }),
				},
				options,
			);
		},
		readFile: (path) => filesystem.readFile({ path }),
		writeFile: (path, content) => filesystem.writeFile({ path, content }),
		mkdir: (path) => filesystem.mkdir({ path, recursive: true }),
		stat: async (path) => ({ isDirectory: (await filesystem.stat({ path })).isDirectory }),
		readdir: (path) => filesystem.readdir({ path }),
		exists: (path) => filesystem.exists({ path }),
	};
}

/**
 * Reads the output retained after `after`. `end` is set once the process has
 * exited. When the actor has dropped output past its retention limit, the read
 * starts at the oldest event it still has.
 */
async function readOutput(
	handle: AgentOSActorHandle,
	process: ProcessId,
	after: number | undefined,
): Promise<RemoteProcessPoll> {
	const chunks: RemoteOutputChunk[] = [];
	let cursor = after;
	while (true) {
		const page = await handle.v1.process.output.read({ process, after: cursor });
		for (const event of page.events) {
			cursor = Number(event.sequence);
			chunks.push({ sequence: cursor, stream: event.stream, data: event.data });
		}
		if (page.hasMore && page.events.length > 0) continue;
		if (!page.end) return { chunks };
		return {
			chunks,
			exit: { exitCode: page.end.signal ? null : page.end.exitCode, timedOut: false },
		};
	}
}
