export {
	type RemoteOutputChunk,
	type RemoteProcess,
	type RemoteProcessPoll,
	runRemoteProcess,
} from "./remote-process.js";

/** The actor context a provider receives: the owning actor's id and a client for the registry. */
export interface SandboxActorContext {
	readonly actorId: string;
	client(): unknown;
}

export interface SandboxExecOptions {
	/** Absolute working directory inside the sandbox. */
	cwd: string;
	env?: Record<string, string>;
	timeoutMs?: number;
	/** Aborting kills the process and rejects `exec` with `Error("aborted")`. */
	signal?: AbortSignal;
	/** Receives stdout and stderr chunks as the sandbox reports them. */
	onData?: (chunk: Uint8Array) => void;
}

export interface SandboxExecResult {
	/** Null when the process was killed by a signal or timed out. */
	exitCode: number | null;
	timedOut: boolean;
	stdout: string;
	stderr: string;
}

/**
 * The file and shell operations an agent's tools run against. All paths are
 * absolute paths inside the sandbox.
 */
export interface Sandbox {
	/** Absolute working directory tools resolve relative paths against. */
	readonly cwd: string;
	exec(command: string, options: SandboxExecOptions): Promise<SandboxExecResult>;
	readFile(path: string): Promise<Uint8Array>;
	writeFile(path: string, content: string): Promise<void>;
	mkdir(path: string): Promise<void>;
	stat(path: string): Promise<{ isDirectory: boolean }>;
	readdir(path: string): Promise<string[]>;
	exists(path: string): Promise<boolean>;
}

/**
 * Creates, reconnects, suspends, and destroys the one sandbox that belongs to
 * an actor. One provider object is shared by every actor of a definition, so
 * it keeps no per-actor state. The caller stores the sandbox id.
 */
export interface SandboxProvider {
	/** Stored next to the sandbox id so a changed provider is detected on wake. */
	readonly name: string;
	/** Provisions a new sandbox and returns its id. */
	create(c: SandboxActorContext): Promise<string>;
	/** Connects to an existing sandbox. Returns undefined when the provider reports that it no longer exists. */
	connect(c: SandboxActorContext, id: string): Promise<Sandbox | undefined>;
	/** Releases resources while the actor sleeps. When omitted, the sandbox keeps running. */
	suspend?(c: SandboxActorContext, id: string): Promise<void>;
	/** Permanently deletes the sandbox when the actor is destroyed. When omitted, the sandbox is left in place. */
	destroy?(c: SandboxActorContext, id: string): Promise<void>;
}
