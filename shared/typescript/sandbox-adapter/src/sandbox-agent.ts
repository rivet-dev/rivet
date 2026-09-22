import { posix } from "node:path";
import {
	SandboxAgent,
	SandboxAgentError,
	type SandboxProvider as SandboxAgentBackend,
	SandboxDestroyedError,
} from "sandbox-agent";
import type { Sandbox, SandboxProvider } from "./index.js";
import { type RemoteOutputChunk, runRemoteProcess } from "./remote-process.js";

/**
 * Runs an agent's file and shell tools in a sandbox started by a sandbox-agent
 * backend, working in the backend's default directory. The sandbox is paused
 * while the actor sleeps when the backend supports pausing, and killed when
 * the actor is destroyed.
 */
export function sandboxAgentProvider(backend: SandboxAgentBackend): SandboxProvider {
	const cwd = backend.defaultCwd;
	if (!cwd || !posix.isAbsolute(cwd)) {
		throw new Error(`sandbox backend ${backend.name} has no absolute default working directory`);
	}
	const { pause, kill } = backend;

	return {
		name: backend.name,
		create: () => backend.create(),
		connect: async (_c, id) => {
			let agent: SandboxAgent;
			try {
				agent = await SandboxAgent.start({
					sandbox: backend,
					sandboxId: `${backend.name}/${id}`,
				});
			} catch (error) {
				if (error instanceof SandboxDestroyedError) return undefined;
				throw error;
			}
			await agent.mkdirFs({ path: cwd });
			return sandboxAgentSandbox(agent, posix.normalize(cwd));
		},
		// Never fall back to destroy here: that would delete the workspace on every sleep.
		suspend: pause ? (_c, id) => pause.call(backend, id) : undefined,
		destroy: (_c, id) => (kill ?? backend.destroy).call(backend, id),
	};
}

function sandboxAgentSandbox(agent: SandboxAgent, cwd: string): Sandbox {
	return {
		cwd,
		exec: async (command, options) => {
			const { id } = await agent.createProcess({
				command: "sh",
				args: ["-c", command],
				cwd: options.cwd,
				env: options.env,
			});
			return runRemoteProcess(
				{
					poll: async (after) => {
						const info = await agent.getProcess(id);
						const logs = await agent.getProcessLogs(id, {
							stream: "combined",
							since: after,
						});
						const chunks = logs.entries.map(
							(entry): RemoteOutputChunk => ({
								sequence: entry.sequence,
								stream: entry.stream === "stderr" ? "stderr" : "stdout",
								data: Buffer.from(
									entry.data,
									entry.encoding === "base64" ? "base64" : "utf8",
								),
							}),
						);
						if (info.status !== "exited") return { chunks };
						return { chunks, exit: { exitCode: info.exitCode ?? null, timedOut: false } };
					},
					kill: async () => {
						await agent.killProcess(id);
					},
				},
				options,
			);
		},
		readFile: (path) => agent.readFsFile({ path }),
		writeFile: async (path, content) => {
			await agent.writeFsFile({ path }, content);
		},
		mkdir: async (path) => {
			await agent.mkdirFs({ path });
		},
		stat: async (path) => ({
			isDirectory: (await agent.statFs({ path })).entryType === "directory",
		}),
		readdir: async (path) => (await agent.listFsEntries({ path })).map((entry) => entry.name),
		exists: async (path) => {
			try {
				await agent.statFs({ path });
				return true;
			} catch (error) {
				if (isNotFound(error)) return false;
				throw error;
			}
		},
	};
}

/** sandbox-agent reports a missing path as an invalid request whose detail says "path not found". */
function isNotFound(error: unknown): boolean {
	return (
		error instanceof SandboxAgentError &&
		(error.problem?.detail ?? "").includes("path not found")
	);
}
