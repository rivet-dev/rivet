import {
	type CreateSandboxFromSnapshotParams,
	Daytona,
	type Sandbox as DaytonaSandbox,
	DaytonaFileNotFoundError,
	DaytonaNotFoundError,
	DaytonaProcessExecutionTimeoutError,
	SandboxState,
} from "@daytonaio/sdk";
import type { Sandbox, SandboxProvider } from "./index.js";

export interface DaytonaProviderOptions {
	/** Defaults to `new Daytona()`, which reads `DAYTONA_API_KEY`. */
	client?: Daytona;
	/** Options for `Daytona.create`. */
	create?: CreateSandboxFromSnapshotParams;
}

/** Runs an agent's file and shell tools in a Daytona sandbox. The sandbox is stopped while the actor sleeps. */
export function daytonaProvider(options: DaytonaProviderOptions = {}): SandboxProvider {
	let client = options.client;
	const daytona = () => {
		client ??= new Daytona();
		return client;
	};
	return {
		name: "daytona",
		create: async () => (await daytona().create(options.create)).id,
		connect: async (_c, id) => {
			let sandbox: DaytonaSandbox;
			try {
				sandbox = await daytona().get(id);
			} catch (error) {
				if (error instanceof DaytonaNotFoundError) return undefined;
				throw error;
			}
			if (sandbox.state !== SandboxState.STARTED) await sandbox.start();
			const cwd = await sandbox.getWorkDir();
			if (!cwd) throw new Error(`daytona sandbox ${id} reported no working directory`);
			return daytonaSandbox(sandbox, cwd);
		},
		suspend: async (_c, id) => {
			await (await daytona().get(id)).stop();
		},
		destroy: async (_c, id) => {
			await (await daytona().get(id)).delete();
		},
	};
}

function daytonaSandbox(sandbox: DaytonaSandbox, cwd: string): Sandbox {
	return {
		cwd,
		exec: async (command, options) => {
			// Daytona's executeCommand has no cancel, so an aborted command runs until
			// it exits or reaches options.timeoutMs.
			let result: Awaited<ReturnType<typeof sandbox.process.executeCommand>>;
			try {
				result = await sandbox.process.executeCommand(
					command,
					options.cwd,
					options.env,
					options.timeoutMs === undefined ? undefined : Math.ceil(options.timeoutMs / 1000),
				);
			} catch (error) {
				// Daytona ends the command when the timeout elapses and throws.
				if (error instanceof DaytonaProcessExecutionTimeoutError) {
					return { exitCode: null, timedOut: true, stdout: "", stderr: "" };
				}
				throw error;
			}
			// Daytona returns stdout and stderr together once the command ends.
			options.onData?.(Buffer.from(result.result));
			return { exitCode: result.exitCode, timedOut: false, stdout: result.result, stderr: "" };
		},
		readFile: (path) => sandbox.fs.downloadFile(path),
		writeFile: (path, content) => sandbox.fs.uploadFile(Buffer.from(content), path),
		mkdir: (path) => sandbox.fs.createFolder(path, "755"),
		stat: async (path) => ({ isDirectory: (await sandbox.fs.getFileDetails(path)).isDir }),
		readdir: async (path) => (await sandbox.fs.listFiles(path)).map((file) => file.name),
		exists: async (path) => {
			try {
				await sandbox.fs.getFileDetails(path);
				return true;
			} catch (error) {
				if (error instanceof DaytonaFileNotFoundError) return false;
				throw error;
			}
		},
	};
}
