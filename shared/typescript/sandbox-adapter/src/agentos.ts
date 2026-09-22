import { posix } from "node:path";
import type { Sandbox, SandboxActorContext, SandboxProvider } from "./index.js";
import { type RemoteOutputChunk, runRemoteProcess } from "./remote-process.js";

const DEFAULT_CWD = "/workspace";

/**
 * The `@rivet-dev/agentos` actor actions this provider calls, typed
 * structurally so this package does not depend on agentOS.
 */
export interface AgentOSActorHandle {
	process: {
		spawn(
			command: string,
			args: string[],
			options: {
				cwd?: string;
				env?: Record<string, string>;
				output: { retainEvents: true };
			},
		): Promise<{ pid: number }>;
		get(pid: number): Promise<{ state: "running" | "exited" }>;
		wait(pid: number): Promise<{
			outcome: "exited" | "signalled" | "timed_out";
			exitCode?: number;
		}>;
		kill(pid: number): Promise<void>;
		readOutput(
			pid: number,
			options?: { after?: number },
		): Promise<{
			events: Array<{
				sequence: number;
				channel: "stdout" | "stderr" | "pty";
				chunk: { encoding: "base64"; data: string };
			}>;
			hasMore: boolean;
		}>;
	};
	filesystem: {
		readFile(path: string): Promise<Uint8Array>;
		writeFile(path: string, content: string | Uint8Array): Promise<void>;
		mkdir(path: string, options?: { recursive?: boolean }): Promise<void>;
		stat(path: string): Promise<{ isDirectory: boolean }>;
		readdir(path: string): Promise<string[]>;
		exists(path: string): Promise<boolean>;
	};
}

export interface AgentOSProviderOptions {
	/** Name of the `agentOS()` actor in the registry. */
	actor: string;
	/** Working directory inside the VM. Defaults to `/workspace`. */
	cwd?: string;
}

/**
 * Runs an agent's file and shell tools in a separate `agentOS()` actor with
 * the key `["sandbox", <actor id>]`. agentOS actors sleep on their own and expose
 * no destroy action, so the provider neither suspends nor deletes the VM actor.
 */
export function agentOSProvider(options: AgentOSProviderOptions): SandboxProvider {
	const actorName = options.actor.trim();
	if (!actorName) {
		throw new Error("agentOSProvider requires the agentOS actor's registry name");
	}
	const cwd = posix.normalize(options.cwd ?? DEFAULT_CWD);
	if (!posix.isAbsolute(cwd)) {
		throw new Error(`agentOSProvider cwd must be an absolute path, received ${cwd}`);
	}
	const vm = (c: SandboxActorContext, id: string): AgentOSActorHandle => {
		const accessor = (c.client() as Record<string, unknown>)[actorName] as
			| { getOrCreate(key: string[]): AgentOSActorHandle }
			| undefined;
		if (!accessor || typeof accessor.getOrCreate !== "function") {
			throw new Error(`registry has no actor named ${JSON.stringify(actorName)}`);
		}
		return accessor.getOrCreate(["sandbox", id]);
	};

	return {
		name: "agentos",
		create: async (c) => c.actorId,
		connect: async (c, id) => {
			const handle = vm(c, id);
			await handle.filesystem.mkdir(cwd, { recursive: true });
			return agentOSSandbox(handle, cwd);
		},
	};
}

function agentOSSandbox(handle: AgentOSActorHandle, cwd: string): Sandbox {
	const { filesystem } = handle;
	return {
		cwd,
		exec: async (command, options) => {
			const { pid } = await handle.process.spawn("sh", ["-c", command], {
				cwd: options.cwd,
				env: options.env,
				output: { retainEvents: true },
			});
			return runRemoteProcess(
				{
					poll: async (after) => {
						// Read the state before the output, so an exited state means
						// the output read after it is complete.
						const { state } = await handle.process.get(pid);
						const chunks = await readAllOutput(handle, pid, after);
						if (state !== "exited") return { chunks };
						const exit = await handle.process.wait(pid);
						return {
							chunks,
							exit: {
								exitCode: exit.outcome === "exited" ? (exit.exitCode ?? null) : null,
								timedOut: exit.outcome === "timed_out",
							},
						};
					},
					kill: () => handle.process.kill(pid),
				},
				options,
			);
		},
		readFile: (path) => filesystem.readFile(path),
		writeFile: (path, content) => filesystem.writeFile(path, content),
		mkdir: (path) => filesystem.mkdir(path, { recursive: true }),
		stat: async (path) => ({ isDirectory: (await filesystem.stat(path)).isDirectory }),
		readdir: (path) => filesystem.readdir(path),
		exists: (path) => filesystem.exists(path),
	};
}

async function readAllOutput(
	handle: AgentOSActorHandle,
	pid: number,
	after: number | undefined,
): Promise<RemoteOutputChunk[]> {
	const chunks: RemoteOutputChunk[] = [];
	let cursor = after;
	while (true) {
		const page = await handle.process.readOutput(pid, { after: cursor });
		for (const event of page.events) {
			cursor = event.sequence;
			if (event.channel === "pty") continue;
			chunks.push({
				sequence: event.sequence,
				stream: event.channel,
				data: Buffer.from(event.chunk.data, "base64"),
			});
		}
		if (!page.hasMore || page.events.length === 0) return chunks;
	}
}
