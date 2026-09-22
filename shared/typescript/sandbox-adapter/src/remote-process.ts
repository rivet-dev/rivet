import { setTimeout as delay } from "node:timers/promises";
import type { SandboxExecOptions, SandboxExecResult } from "./index.js";

const POLL_INTERVAL_MS = 100;

export interface RemoteOutputChunk {
	sequence: number;
	stream: "stdout" | "stderr";
	data: Uint8Array;
}

export interface RemoteProcessPoll {
	/** Output with a sequence number greater than the one passed to `poll`. */
	chunks: RemoteOutputChunk[];
	/** Set once the process has exited and `chunks` holds all of its remaining output. */
	exit?: { exitCode: number | null; timedOut: boolean };
}

/** A process started in a remote sandbox that retains its output by sequence number. */
export interface RemoteProcess {
	poll(after: number | undefined): Promise<RemoteProcessPoll>;
	kill(): Promise<void>;
}

/**
 * Streams a remote process's output until it exits, kills it on abort or
 * timeout, and collects stdout and stderr. The agentOS actions keep
 * output by sequence number but give an actor no exit notification it can
 * await, so this reads on an interval.
 */
export async function runRemoteProcess(
	process: RemoteProcess,
	options: Pick<SandboxExecOptions, "timeoutMs" | "signal" | "onData">,
): Promise<SandboxExecResult> {
	const stdout: Uint8Array[] = [];
	const stderr: Uint8Array[] = [];
	const result = (exitCode: number | null, timedOut: boolean): SandboxExecResult => ({
		exitCode,
		timedOut,
		stdout: Buffer.concat(stdout).toString(),
		stderr: Buffer.concat(stderr).toString(),
	});
	const deadline = options.timeoutMs ? Date.now() + options.timeoutMs : undefined;
	let after: number | undefined;

	while (true) {
		if (options.signal?.aborted) {
			await killQuietly(process);
			throw new Error("aborted");
		}
		const { chunks, exit } = await process.poll(after);
		for (const chunk of [...chunks].sort((a, b) => a.sequence - b.sequence)) {
			if (after !== undefined && chunk.sequence <= after) continue;
			after = chunk.sequence;
			(chunk.stream === "stdout" ? stdout : stderr).push(chunk.data);
			options.onData?.(chunk.data);
		}
		if (exit) {
			return result(exit.exitCode, exit.timedOut);
		}
		if (deadline !== undefined && Date.now() >= deadline) {
			await killQuietly(process);
			return result(null, true);
		}
		await delay(POLL_INTERVAL_MS, undefined, { signal: options.signal }).catch(
			() => {},
		);
	}
}

async function killQuietly(process: RemoteProcess): Promise<void> {
	try {
		await process.kill();
	} catch {
	}
}
