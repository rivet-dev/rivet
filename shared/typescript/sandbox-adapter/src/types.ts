export type SandboxBinding =
	| null
	| boolean
	| number
	| string
	| SandboxBinding[]
	| { [key: string]: SandboxBinding };

export interface SandboxActorContext {
	readonly actorId: string;
	readonly key: readonly string[];
	client(): unknown;
}

export interface SandboxConnectOptions {
	/** Stable identity for this sandbox. */
	id: string;
	/** Previously persisted provider binding, if this sandbox is reconnecting. */
	binding?: SandboxBinding;
	/** Requested working directory. The adapter may return a different one. */
	cwd?: string;
}

export interface SandboxLifecycleOptions extends SandboxConnectOptions {
	/** Connection returned by the current actor generation, when available. */
	sandbox?: Sandbox;
}

export interface SandboxAdapter<
	TContext extends SandboxActorContext = SandboxActorContext,
> {
	/** Provision or reconnect to a sandbox. This must be idempotent for `id`. */
	connect(
		context: TContext,
		options: SandboxConnectOptions,
	): Promise<Sandbox>;
	/** Release live resources while preserving the sandbox for a later wake. */
	suspend?(
		context: TContext,
		options: SandboxLifecycleOptions,
	): Promise<void>;
	/** Permanently delete the sandbox. */
	destroy?(
		context: TContext,
		options: SandboxLifecycleOptions,
	): Promise<void>;
}

export interface SandboxExecOptions {
	cwd?: string;
	env?: Record<string, string>;
	stdin?: string | Uint8Array;
	timeoutMs?: number;
	signal?: AbortSignal;
	onOutput?: (event: SandboxOutputEvent) => void;
	/** Maximum combined stdout/stderr retained by the adapter. */
	maxOutputBytes?: number;
}

export interface SandboxExecResult {
	exitCode: number | null;
	stdout: string;
	stderr: string;
	/** Set when the sandbox reports a signal or timeout separately from exit. */
	outcome?: "exited" | "signalled" | "timed_out";
	signal?: string;
	/** True when output was capped and the process was stopped. */
	truncated?: boolean;
}

export interface SandboxSpawnOptions extends SandboxExecOptions {
	/** Retain output so callers can reconnect and continue reading it. */
	retainOutput?: boolean;
}

export interface SandboxOutputEvent {
	sequence: number;
	stream: "stdout" | "stderr" | "pty";
	data: Uint8Array;
	timestampMs?: number;
}

export interface SandboxOutputPage {
	events: SandboxOutputEvent[];
	/** Sequence to pass as `after` on the next read. */
	nextSequence: number;
	hasMore: boolean;
	truncated: boolean;
}

export interface SandboxProcessExit {
	exitCode: number | null;
	outcome: "exited" | "signalled" | "timed_out";
	signal?: string;
}

/**
 * Pull-based process handle. A pull cursor survives actor action boundaries and
 * can be adapted to streams without retaining callbacks in a sandbox provider.
 */
export interface SandboxProcess {
	readonly pid: number | string;
	readOutput(options?: {
		after?: number;
		/** Maximum bytes to return in this page. */
		maxBytes?: number;
		/** Maximum events to return in this page. */
		maxEvents?: number;
	}): Promise<SandboxOutputPage>;
	wait(): Promise<SandboxProcessExit>;
	writeStdin(data: string | Uint8Array): Promise<void>;
	closeStdin(): Promise<void>;
	kill(signal?: string): Promise<void>;
}

export interface SandboxFileStat {
	type: "file" | "directory" | "symlink" | "other";
	size: number;
	mode?: number;
	mtimeMs?: number;
}

export interface Sandbox {
	/** Opaque provider identity that callers should persist durably. */
	readonly binding: SandboxBinding;
	/** Absolute working directory inside the sandbox. */
	readonly cwd: string;
	exec(command: string, options?: SandboxExecOptions): Promise<SandboxExecResult>;
	spawn(
		command: string,
		args?: readonly string[],
		options?: SandboxSpawnOptions,
	): Promise<SandboxProcess>;
	readFile(path: string, options?: { maxBytes?: number }): Promise<Uint8Array>;
	writeFile(path: string, content: string | Uint8Array): Promise<void>;
	stat(path: string): Promise<SandboxFileStat>;
	readdir(path: string): Promise<string[]>;
	exists(path: string): Promise<boolean>;
	mkdir(path: string, options?: { recursive?: boolean }): Promise<void>;
	remove(path: string, options?: { recursive?: boolean; force?: boolean }): Promise<void>;
}
