import { spawn } from "node:child_process";
import { randomUUID } from "node:crypto";
import { mkdir, readdir, readFile, rm, stat, writeFile } from "node:fs/promises";
import { join } from "node:path";
import {
	type RemoteOutputChunk,
	type RemoteProcess,
	runRemoteProcess,
	type Sandbox,
	type SandboxProvider,
} from "@rivet-dev/sandbox-adapter";

/**
 * A sandbox provider whose sandboxes are directories under `root`. Commands
 * run as local processes whose output is read by sequence number, the same
 * way the agentOS and sandbox-agent providers read remote processes.
 */
export function localSandboxProvider(root: string): SandboxProvider {
	const directory = (id: string) => join(root, id);
	return {
		name: "local",
		create: async () => {
			const id = randomUUID();
			await mkdir(directory(id), { recursive: true });
			return id;
		},
		connect: async (_c, id) =>
			(await exists(directory(id))) ? localSandbox(directory(id)) : undefined,
		destroy: async (_c, id) => {
			await rm(directory(id), { recursive: true, force: true });
		},
	};
}

function localSandbox(cwd: string): Sandbox {
	return {
		cwd,
		exec: (command, options) =>
			runRemoteProcess(startLocalProcess(command, options.cwd, options.env), options),
		readFile: (path) => readFile(path),
		writeFile: (path, content) => writeFile(path, content),
		mkdir: async (path) => {
			await mkdir(path, { recursive: true });
		},
		stat: async (path) => ({ isDirectory: (await stat(path)).isDirectory() }),
		readdir: (path) => readdir(path),
		exists,
	};
}

function exists(path: string): Promise<boolean> {
	return stat(path).then(
		() => true,
		() => false,
	);
}

function startLocalProcess(
	command: string,
	cwd: string,
	env: Record<string, string> | undefined,
): RemoteProcess {
	const child = spawn("/bin/sh", ["-c", command], {
		cwd,
		env: { ...process.env, ...env },
	});
	const output: RemoteOutputChunk[] = [];
	let sequence = 0;
	child.stdout.on("data", (data: Buffer) => {
		output.push({ sequence: sequence++, stream: "stdout", data });
	});
	child.stderr.on("data", (data: Buffer) => {
		output.push({ sequence: sequence++, stream: "stderr", data });
	});
	let exit: { exitCode: number | null; timedOut: boolean } | undefined;
	child.on("close", (exitCode) => {
		exit = { exitCode, timedOut: false };
	});
	return {
		poll: async (after) => {
			const exited = exit;
			return {
				chunks: output.filter((chunk) => after === undefined || chunk.sequence > after),
				exit: exited,
			};
		},
		kill: async () => {
			child.kill("SIGKILL");
		},
	};
}
