import { readFileSync, realpathSync } from "node:fs";
import { join } from "node:path";
import {
	allowAll,
	createInMemoryFileSystem,
	createKernel,
	type FsMount,
	type Kernel,
	type KernelExecOptions,
	type KernelExecResult,
	type ProcessInfo as KernelProcessInfo,
	type KernelSpawnOptions,
	type ManagedProcess,
	type OpenShellOptions,
	type Permissions,
	type ShellHandle,
	type VirtualFileSystem,
	type VirtualStat,
} from "@secure-exec/core";
import { type ToolKit, validateToolkits } from "./host-tools.js";
import { generateToolReference } from "./host-tools-prompt.js";
import {
	startHostToolsServer,
	type HostToolsServer,
} from "./host-tools-server.js";
import {
	createShimFilesystem,
	generateMasterShim,
	generateToolkitShim,
} from "./host-tools-shims.js";

/**
 * Context describing the agent that a filesystem backend is being created for.
 * Filesystem drivers use this to derive per-agent storage prefixes automatically.
 */
export interface AgentOsContext {
	/** Unique key identifying this agent instance (e.g. actor ID, user ID, session ID). */
	key: string;
	/** Arbitrary metadata about the agent. */
	metadata?: Record<string, string>;
}

/** Process tree node: extends kernel ProcessInfo with child references. */
export interface ProcessTreeNode extends KernelProcessInfo {
	children: ProcessTreeNode[];
}

/** A directory entry with metadata. */
export interface DirEntry {
	/** Absolute path to the entry. */
	path: string;
	type: "file" | "directory" | "symlink";
	size: number;
}

/** Options for readdirRecursive(). */
export interface ReaddirRecursiveOptions {
	/** Maximum depth to recurse (0 = only immediate children). */
	maxDepth?: number;
	/** Directory names to skip. */
	exclude?: string[];
}

/** Entry for batch write operations. */
export interface BatchWriteEntry {
	path: string;
	content: string | Uint8Array;
}

/** Result of a single file in a batch write. */
export interface BatchWriteResult {
	path: string;
	success: boolean;
	error?: string;
}

/** Result of a single file in a batch read. */
export interface BatchReadResult {
	path: string;
	content: Uint8Array | null;
	error?: string;
}

/** Entry in the agent registry, describing an available agent type. */
export interface AgentRegistryEntry {
	id: AgentType;
	acpAdapter: string;
	agentPackage: string;
	installed: boolean;
}

import {
	createNodeHostNetworkAdapter,
	createNodeRuntime,
} from "@secure-exec/nodejs";
import { createPythonRuntime } from "@secure-exec/python";
import { createWasmVmRuntime } from "@secure-exec/wasmvm";
import { AcpClient } from "./acp-client.js";
import { AGENT_CONFIGS, type AgentConfig, type AgentType } from "./agents.js";
import {
	type SoftwareInput,
	type SoftwareRoot,
	processSoftware,
} from "./packages.js";
import { CronManager } from "./cron/cron-manager.js";
import type { ScheduleDriver } from "./cron/schedule-driver.js";
import { TimerScheduleDriver } from "./cron/timer-driver.js";
import type {
	CronEvent,
	CronEventHandler,
	CronJob,
	CronJobInfo,
	CronJobOptions,
} from "./cron/types.js";
import { getOsInstructions } from "./os-instructions.js";
import {
	Session,
	type SessionInitData,
	type AgentCapabilities,
	type AgentInfo,
	type GetEventsOptions,
	type PermissionReply,
	type SequencedEvent,
	type SessionConfigOption,
	type SessionEventHandler,
	type SessionModeState,
	type PermissionRequestHandler,
} from "./session.js";
import type { JsonRpcResponse } from "./protocol.js";
import { createStdoutLineIterable } from "./stdout-lines.js";

/** Configuration for mounting a filesystem driver at a path. */
export interface MountConfig {
	/** Path inside the VM to mount at. */
	path: string;
	/** The filesystem driver to mount. */
	driver: VirtualFileSystem;
	/** If true, write operations throw EROFS. */
	readOnly?: boolean;
}

export interface AgentOsOptions {
	/**
	 * Software to install in the VM. Each entry provides agents, tools,
	 * or WASM commands. Any object with a `commandDir` property (e.g.,
	 * registry packages like @rivet-dev/agent-os-coreutils) is treated
	 * as a WASM command source automatically. Arrays are flattened, so
	 * meta-packages that export arrays of sub-packages work directly.
	 */
	software?: SoftwareInput[];
	/** Loopback ports to exempt from SSRF checks (for testing with host-side mock servers). */
	loopbackExemptPorts?: number[];
	/**
	 * Host-side CWD for module access resolution. Sets the directory whose
	 * node_modules are projected into the VM at /root/node_modules/.
	 * Defaults to process.cwd().
	 */
	moduleAccessCwd?: string;
	/** Filesystems to mount at boot time. */
	mounts?: MountConfig[];
	/** Additional instructions appended to the base OS instructions written to /etc/agentos/instructions.md. */
	additionalInstructions?: string;
	/** Custom schedule driver for cron jobs. Defaults to TimerScheduleDriver. */
	scheduleDriver?: ScheduleDriver;
	/** Host-side toolkits available to agents inside the VM. */
	toolKits?: ToolKit[];
	/**
	 * Custom permission policy for the kernel. Controls access to filesystem,
	 * network, child process, and environment operations. Defaults to allowAll.
	 */
	permissions?: Permissions;
}

/** Configuration for a local MCP server (spawned as a child process). */
export interface McpServerConfigLocal {
	type: "local";
	/** Command to launch the MCP server. */
	command: string;
	/** Arguments for the command. */
	args?: string[];
	/** Environment variables for the server process. */
	env?: Record<string, string>;
}

/** Configuration for a remote MCP server (connected via URL). */
export interface McpServerConfigRemote {
	type: "remote";
	/** URL of the remote MCP server. */
	url: string;
	/** HTTP headers to include in requests to the server. */
	headers?: Record<string, string>;
}

export type McpServerConfig = McpServerConfigLocal | McpServerConfigRemote;

export interface CreateSessionOptions {
	/** Working directory for the agent session inside the VM. */
	cwd?: string;
	/** Environment variables to pass to the agent process. */
	env?: Record<string, string>;
	/** MCP servers to make available to the agent during the session. */
	mcpServers?: McpServerConfig[];
	/** Skip OS instructions injection entirely (default false). */
	skipOsInstructions?: boolean;
	/** Additional instructions appended to the base OS instructions. */
	additionalInstructions?: string;
}

export interface SessionInfo {
	sessionId: string;
	agentType: string;
}

/** Information about a process spawned via AgentOs.spawn(). */
export interface SpawnedProcessInfo {
	pid: number;
	command: string;
	args: string[];
	running: boolean;
	exitCode: number | null;
}

export class AgentOs {
	readonly kernel: Kernel;
	private _sessions = new Map<string, Session>();
	private _processes = new Map<
		number,
		{
			proc: ManagedProcess;
			command: string;
			args: string[];
			stdoutHandlers: Set<(data: Uint8Array) => void>;
			stderrHandlers: Set<(data: Uint8Array) => void>;
			exitHandlers: Set<(exitCode: number) => void>;
		}
	>();
	private _shells = new Map<
		string,
		{
			handle: ShellHandle;
			dataHandlers: Set<(data: Uint8Array) => void>;
		}
	>();
	private _shellCounter = 0;
	private _moduleAccessCwd: string;
	private _softwareRoots: SoftwareRoot[];
	private _softwareAgentConfigs: Map<string, AgentConfig>;
	private _cronManager!: CronManager;
	private _toolsServer: HostToolsServer | null = null;
	private _toolKits: ToolKit[] = [];
	private _shimFs: ReturnType<typeof createInMemoryFileSystem> | null = null;
	private _env: Record<string, string>;

	private constructor(
		kernel: Kernel,
		moduleAccessCwd: string,
		softwareRoots: SoftwareRoot[],
		softwareAgentConfigs: Map<string, AgentConfig>,
		env: Record<string, string>,
	) {
		this.kernel = kernel;
		this._moduleAccessCwd = moduleAccessCwd;
		this._softwareRoots = softwareRoots;
		this._softwareAgentConfigs = softwareAgentConfigs;
		this._env = env;
	}

	static async create(options?: AgentOsOptions): Promise<AgentOs> {
		const filesystem = createInMemoryFileSystem();
		const hostNetworkAdapter = createNodeHostNetworkAdapter();
		const moduleAccessCwd = options?.moduleAccessCwd ?? process.cwd();

		// Process software descriptors to collect WASM dirs, module roots, and agent configs.
		const processed = processSoftware(options?.software ?? []);

		const mounts = options?.mounts?.map((m) => ({
			path: m.path,
			fs: m.driver,
			readOnly: m.readOnly,
		}));

		// Start host tools RPC server before kernel creation so the port
		// can be included in the kernel env and loopback exemptions.
		let toolsServer: HostToolsServer | null = null;
		const toolKits = options?.toolKits;
		if (toolKits && toolKits.length > 0) {
			validateToolkits(toolKits);
			toolsServer = await startHostToolsServer(toolKits);
		}

		const loopbackExemptPorts = [
			...(options?.loopbackExemptPorts ?? []),
			...(toolsServer ? [toolsServer.port] : []),
		];

		const env: Record<string, string> = {
			HOME: "/home/user",
			USER: "user",
			PATH: "/usr/local/bin:/usr/bin:/bin",
		};
		if (toolsServer) {
			env.AGENTOS_TOOLS_PORT = String(toolsServer.port);
		}

		const kernel = createKernel({
			filesystem,
			hostNetworkAdapter,
			permissions: options?.permissions ?? allowAll,
			env,
			cwd: "/home/user",
			mounts,
		});

		// Mount OS instructions at /etc/agentos/ as a read-only filesystem
		// so agents cannot tamper with their own instructions.
		const etcAgentosFs = createInMemoryFileSystem();
		const instructions = getOsInstructions(options?.additionalInstructions);
		await etcAgentosFs.writeFile("instructions.md", instructions);
		kernel.mountFs("/etc/agentos", etcAgentosFs, { readOnly: true });

		// Mount CLI shims for host tools at /usr/local/bin so agents can
		// invoke tools via shell commands (agentos-{name} <tool> ...).
		let shimFs: ReturnType<typeof createInMemoryFileSystem> | null = null;
		if (toolKits && toolKits.length > 0) {
			shimFs = await createShimFilesystem(toolKits);
			kernel.mountFs("/usr/local/bin", shimFs, { readOnly: true });
		}

		await kernel.mount(
			createWasmVmRuntime(
				processed.commandDirs.length > 0
					? { commandDirs: processed.commandDirs }
					: undefined,
			),
		);
		await kernel.mount(
			createNodeRuntime({
				loopbackExemptPorts,
				moduleAccessCwd,
				packageRoots: processed.softwareRoots.length > 0
					? processed.softwareRoots
					: undefined,
			}),
		);
		await kernel.mount(createPythonRuntime());

		const vm = new AgentOs(
			kernel,
			moduleAccessCwd,
			processed.softwareRoots,
			processed.agentConfigs,
			env,
		);
		vm._toolsServer = toolsServer;
		vm._toolKits = toolKits ?? [];
		vm._shimFs = shimFs;
		vm._cronManager = new CronManager(
			vm,
			options?.scheduleDriver ?? new TimerScheduleDriver(),
		);

		return vm;
	}

	async exec(
		command: string,
		options?: KernelExecOptions,
	): Promise<KernelExecResult> {
		return this.kernel.exec(command, options);
	}

	spawn(
		command: string,
		args: string[],
		options?: KernelSpawnOptions,
	): { pid: number } {
		const stdoutHandlers = new Set<(data: Uint8Array) => void>();
		const stderrHandlers = new Set<(data: Uint8Array) => void>();
		const exitHandlers = new Set<(exitCode: number) => void>();

		// Include caller-provided callbacks in the initial handler sets.
		if (options?.onStdout) stdoutHandlers.add(options.onStdout);
		if (options?.onStderr) stderrHandlers.add(options.onStderr);

		const proc = this.kernel.spawn(command, args, {
			...options,
			onStdout: (data) => {
				for (const h of stdoutHandlers) h(data);
			},
			onStderr: (data) => {
				for (const h of stderrHandlers) h(data);
			},
		});

		const entry = {
			proc,
			command,
			args,
			stdoutHandlers,
			stderrHandlers,
			exitHandlers,
		};
		this._processes.set(proc.pid, entry);

		// Monitor exit and notify handlers.
		proc.wait().then((code) => {
			for (const h of exitHandlers) h(code);
		});

		return { pid: proc.pid };
	}

	/** Write data to a process's stdin. */
	writeProcessStdin(pid: number, data: string | Uint8Array): void {
		const entry = this._processes.get(pid);
		if (!entry) throw new Error(`Process not found: ${pid}`);
		entry.proc.writeStdin(data);
	}

	/** Close a process's stdin stream. */
	closeProcessStdin(pid: number): void {
		const entry = this._processes.get(pid);
		if (!entry) throw new Error(`Process not found: ${pid}`);
		entry.proc.closeStdin();
	}

	/** Subscribe to stdout data from a process. Returns an unsubscribe function. */
	onProcessStdout(
		pid: number,
		handler: (data: Uint8Array) => void,
	): () => void {
		const entry = this._processes.get(pid);
		if (!entry) throw new Error(`Process not found: ${pid}`);
		entry.stdoutHandlers.add(handler);
		return () => {
			entry.stdoutHandlers.delete(handler);
		};
	}

	/** Subscribe to stderr data from a process. Returns an unsubscribe function. */
	onProcessStderr(
		pid: number,
		handler: (data: Uint8Array) => void,
	): () => void {
		const entry = this._processes.get(pid);
		if (!entry) throw new Error(`Process not found: ${pid}`);
		entry.stderrHandlers.add(handler);
		return () => {
			entry.stderrHandlers.delete(handler);
		};
	}

	/** Subscribe to process exit. Returns an unsubscribe function. */
	onProcessExit(
		pid: number,
		handler: (exitCode: number) => void,
	): () => void {
		const entry = this._processes.get(pid);
		if (!entry) throw new Error(`Process not found: ${pid}`);
		// If already exited, call immediately.
		if (entry.proc.exitCode !== null) {
			handler(entry.proc.exitCode);
			return () => {};
		}
		entry.exitHandlers.add(handler);
		return () => {
			entry.exitHandlers.delete(handler);
		};
	}

	/** Wait for a process to exit. Returns the exit code. */
	waitProcess(pid: number): Promise<number> {
		const entry = this._processes.get(pid);
		if (!entry) throw new Error(`Process not found: ${pid}`);
		return entry.proc.wait();
	}

	async readFile(path: string): Promise<Uint8Array> {
		return this.kernel.readFile(path);
	}

	async writeFile(path: string, content: string | Uint8Array): Promise<void> {
		return this.kernel.writeFile(path, content);
	}

	async writeFiles(entries: BatchWriteEntry[]): Promise<BatchWriteResult[]> {
		const results: BatchWriteResult[] = [];
		for (const entry of entries) {
			try {
				// Create parent directories as needed
				const parentDir = entry.path.substring(
					0,
					entry.path.lastIndexOf("/"),
				);
				if (parentDir) {
					await this._mkdirp(parentDir);
				}
				await this.kernel.writeFile(entry.path, entry.content);
				results.push({ path: entry.path, success: true });
			} catch (err: unknown) {
				results.push({
					path: entry.path,
					success: false,
					error: err instanceof Error ? err.message : String(err),
				});
			}
		}
		return results;
	}

	async readFiles(paths: string[]): Promise<BatchReadResult[]> {
		const results: BatchReadResult[] = [];
		for (const path of paths) {
			try {
				const content = await this.kernel.readFile(path);
				results.push({ path, content });
			} catch (err: unknown) {
				results.push({
					path,
					content: null,
					error: err instanceof Error ? err.message : String(err),
				});
			}
		}
		return results;
	}

	/** Recursively create directories (mkdir -p). */
	private async _mkdirp(path: string): Promise<void> {
		const parts = path.split("/").filter(Boolean);
		let current = "";
		for (const part of parts) {
			current += `/${part}`;
			if (!(await this.kernel.exists(current))) {
				await this.kernel.mkdir(current);
			}
		}
	}

	async mkdir(path: string): Promise<void> {
		return this.kernel.mkdir(path);
	}

	async readdir(path: string): Promise<string[]> {
		return this.kernel.readdir(path);
	}

	async readdirRecursive(
		path: string,
		options?: ReaddirRecursiveOptions,
	): Promise<DirEntry[]> {
		const maxDepth = options?.maxDepth;
		const exclude = options?.exclude ? new Set(options.exclude) : undefined;
		const results: DirEntry[] = [];

		// BFS queue: [dirPath, currentDepth]
		const queue: [string, number][] = [[path, 0]];

		while (queue.length > 0) {
			const item = queue.shift();
			if (!item) break;
			const [dirPath, depth] = item;
			const entries = await this.kernel.readdir(dirPath);

			for (const name of entries) {
				if (name === "." || name === "..") continue;
				if (exclude?.has(name)) continue;

				const fullPath =
					dirPath === "/" ? `/${name}` : `${dirPath}/${name}`;
				const s = await this.kernel.stat(fullPath);

				if (s.isSymbolicLink) {
					results.push({
						path: fullPath,
						type: "symlink",
						size: s.size,
					});
				} else if (s.isDirectory) {
					results.push({
						path: fullPath,
						type: "directory",
						size: s.size,
					});
					if (maxDepth === undefined || depth < maxDepth) {
						queue.push([fullPath, depth + 1]);
					}
				} else {
					results.push({
						path: fullPath,
						type: "file",
						size: s.size,
					});
				}
			}
		}

		return results;
	}

	async stat(path: string): Promise<VirtualStat> {
		return this.kernel.stat(path);
	}

	async exists(path: string): Promise<boolean> {
		return this.kernel.exists(path);
	}

	mountFs(path: string, driver: VirtualFileSystem, options?: { readOnly?: boolean }): void {
		this.kernel.mountFs(path, driver, { readOnly: options?.readOnly });
	}

	unmountFs(path: string): void {
		this.kernel.unmountFs(path);
	}

	async move(from: string, to: string): Promise<void> {
		return this.kernel.rename(from, to);
	}

	async delete(
		path: string,
		options?: { recursive?: boolean },
	): Promise<void> {
		const s = await this.kernel.stat(path);
		if (s.isDirectory) {
			if (options?.recursive) {
				const entries = await this.kernel.readdir(path);
				for (const entry of entries) {
					if (entry === "." || entry === "..") continue;
					await this.delete(`${path}/${entry}`, { recursive: true });
				}
			}
			return this.kernel.removeDir(path);
		}
		return this.kernel.removeFile(path);
	}

	async fetch(port: number, request: Request): Promise<Response> {
		const url = new URL(request.url);
		url.hostname = "127.0.0.1";
		url.port = String(port);
		url.protocol = "http:";

		return globalThis.fetch(
			new Request(url, {
				method: request.method,
				headers: request.headers,
				body: request.body,
				redirect: request.redirect,
				signal: request.signal,
			}),
		);
	}

	openShell(options?: OpenShellOptions): { shellId: string } {
		const shellId = `shell-${++this._shellCounter}`;
		const dataHandlers = new Set<(data: Uint8Array) => void>();

		const handle = this.kernel.openShell(options);
		handle.onData = (data) => {
			for (const h of dataHandlers) h(data);
		};

		this._shells.set(shellId, { handle, dataHandlers });
		return { shellId };
	}

	/** Write data to a shell's PTY input. */
	writeShell(shellId: string, data: string | Uint8Array): void {
		const entry = this._shells.get(shellId);
		if (!entry) throw new Error(`Shell not found: ${shellId}`);
		entry.handle.write(data);
	}

	/** Subscribe to data output from a shell. Returns an unsubscribe function. */
	onShellData(
		shellId: string,
		handler: (data: Uint8Array) => void,
	): () => void {
		const entry = this._shells.get(shellId);
		if (!entry) throw new Error(`Shell not found: ${shellId}`);
		entry.dataHandlers.add(handler);
		return () => {
			entry.dataHandlers.delete(handler);
		};
	}

	/** Notify a shell of terminal resize. */
	resizeShell(shellId: string, cols: number, rows: number): void {
		const entry = this._shells.get(shellId);
		if (!entry) throw new Error(`Shell not found: ${shellId}`);
		entry.handle.resize(cols, rows);
	}

	/** Kill a shell process and remove it from tracking. */
	closeShell(shellId: string): void {
		const entry = this._shells.get(shellId);
		if (!entry) throw new Error(`Shell not found: ${shellId}`);
		entry.handle.kill();
		this._shells.delete(shellId);
	}

	/** Returns info about all processes spawned via spawn(). */
	listProcesses(): SpawnedProcessInfo[] {
		return [...this._processes.values()].map(({ proc, command, args }) => ({
			pid: proc.pid,
			command,
			args,
			running: proc.exitCode === null,
			exitCode: proc.exitCode,
		}));
	}

	/** Returns all kernel processes across all runtimes (WASM, Node, Python). */
	allProcesses(): KernelProcessInfo[] {
		return [...this.kernel.processes.values()];
	}

	/** Returns processes organized as a tree using ppid relationships. */
	processTree(): ProcessTreeNode[] {
		const all = this.allProcesses();
		const nodeMap = new Map<number, ProcessTreeNode>();

		// Index: create a tree node for each process
		for (const proc of all) {
			nodeMap.set(proc.pid, { ...proc, children: [] });
		}

		// Wire: attach each node to its parent
		const roots: ProcessTreeNode[] = [];
		for (const node of nodeMap.values()) {
			const parent = nodeMap.get(node.ppid);
			if (parent) {
				parent.children.push(node);
			} else {
				roots.push(node);
			}
		}

		return roots;
	}

	/** Returns info about a specific process by PID. Throws if not found. */
	getProcess(pid: number): SpawnedProcessInfo {
		const entry = this._processes.get(pid);
		if (!entry) {
			throw new Error(`Process not found: ${pid}`);
		}
		return {
			pid: entry.proc.pid,
			command: entry.command,
			args: entry.args,
			running: entry.proc.exitCode === null,
			exitCode: entry.proc.exitCode,
		};
	}

	/** Send SIGTERM to gracefully stop a process. No-op if already exited. */
	stopProcess(pid: number): void {
		const entry = this._processes.get(pid);
		if (!entry) {
			throw new Error(`Process not found: ${pid}`);
		}
		if (entry.proc.exitCode !== null) return;
		entry.proc.kill();
	}

	/** Send SIGKILL to force-kill a process. No-op if already exited. */
	killProcess(pid: number): void {
		const entry = this._processes.get(pid);
		if (!entry) {
			throw new Error(`Process not found: ${pid}`);
		}
		if (entry.proc.exitCode !== null) return;
		entry.proc.kill(9);
	}

	/** Returns all active sessions with their IDs and agent types. */
	listSessions(): SessionInfo[] {
		return [...this._sessions.values()].map((s) => ({
			sessionId: s.sessionId,
			agentType: s.agentType,
		}));
	}

	/** Internal helper: retrieve a session or throw. */
	private _requireSession(sessionId: string): Session {
		const session = this._sessions.get(sessionId);
		if (!session) {
			throw new Error(`Session not found: ${sessionId}`);
		}
		return session;
	}

	/** Returns all registered agents with their installation status. */
	listAgents(): AgentRegistryEntry[] {
		// Collect all agent IDs from both package configs and hardcoded configs.
		const allIds = new Set<string>([
			...this._softwareAgentConfigs.keys(),
			...Object.keys(AGENT_CONFIGS),
		]);

		return [...allIds].map((id) => {
			const config = this._resolveAgentConfig(id);
			if (!config) return null;

			let installed = false;
			try {
				// Check package roots first, then CWD-based node_modules.
				const vmPrefix = `/root/node_modules/${config.acpAdapter}`;
				let hostPkgJsonPath: string | null = null;
				for (const root of this._softwareRoots) {
					if (root.vmPath === vmPrefix) {
						hostPkgJsonPath = join(root.hostPath, "package.json");
						break;
					}
				}
				if (!hostPkgJsonPath) {
					hostPkgJsonPath = join(
						this._moduleAccessCwd,
						"node_modules",
						config.acpAdapter,
						"package.json",
					);
				}
				readFileSync(hostPkgJsonPath);
				installed = true;
			} catch {
				// Package not installed
			}
			return {
				id: id as AgentType,
				acpAdapter: config.acpAdapter,
				agentPackage: config.agentPackage,
				installed,
			};
		}).filter((entry): entry is AgentRegistryEntry => entry !== null);
	}

	/**
	 * Spawn an ACP-compatible coding agent inside the VM and return a Session.
	 *
	 * 1. Resolves the adapter binary from mounted node_modules
	 * 2. Spawns it with streaming stdin and stdout capture
	 * 3. Sends initialize + session/new
	 * 4. Returns a Session for prompt/cancel/close
	 */
	async createSession(
		agentType: AgentType | string,
		options?: CreateSessionOptions,
	): Promise<{ sessionId: string }> {
		const config = this._resolveAgentConfig(agentType);
		if (!config) {
			throw new Error(`Unknown agent type: ${agentType}`);
		}

		// Resolve the adapter's CLI entry point from mounted node_modules
		const binPath = this._resolveAdapterBin(config.acpAdapter);

		// Generate tool reference from VM-level toolkits. This is always
		// injected into the agent prompt, even when skipOsInstructions is true.
		const toolReference =
			this._toolKits.length > 0
				? generateToolReference(this._toolKits)
				: undefined;

		// Prepare OS instructions injection. When skipOsInstructions is true,
		// the base OS instructions are skipped but tool docs are still injected.
		let extraArgs: string[] = [];
		let extraEnv: Record<string, string> = {};
		if (config.prepareInstructions) {
			const cwd = options?.cwd ?? "/home/user";
			const skipBase = options?.skipOsInstructions ?? false;
			const hasToolRef = !!toolReference;

			if (!skipBase || hasToolRef) {
				const prepared = await config.prepareInstructions(
					this.kernel,
					cwd,
					skipBase ? undefined : options?.additionalInstructions,
					{ toolReference, skipBase },
				);
				if (prepared.args) extraArgs = prepared.args;
				if (prepared.env) extraEnv = prepared.env;
			}
		}

		// Create stdout line iterable wired via onStdout callback
		const { iterable, onStdout } = createStdoutLineIterable();

		// Spawn the adapter inside the VM with streaming stdin.
		// Use the public spawn() so the onStdout callback is properly wired
		// through the process handler multiplexer.
		// Env priority (lowest to highest): defaultEnv, prepareInstructions env, user env
		const { pid } = this.spawn("node", [binPath, ...extraArgs], {
			streamStdin: true,
			onStdout,
			env: { ...config.defaultEnv, ...extraEnv, ...options?.env },
			cwd: options?.cwd,
		});
		const proc = this._processes.get(pid)!.proc;

		// Wire up ACP client
		const client = new AcpClient(proc, iterable);

		// Initialize the ACP protocol
		const initResponse = await client.request("initialize", {
			protocolVersion: 1,
			clientCapabilities: {},
		});
		if (initResponse.error) {
			client.close();
			throw new Error(
				`ACP initialize failed: ${initResponse.error.message}`,
			);
		}

		// Create a new session
		const sessionResponse = await client.request("session/new", {
			cwd: options?.cwd ?? "/home/user",
			mcpServers: options?.mcpServers ?? [],
		});
		if (sessionResponse.error) {
			client.close();
			throw new Error(
				`ACP session/new failed: ${sessionResponse.error.message}`,
			);
		}

		const sessionId = (sessionResponse.result as { sessionId: string })
			.sessionId;

		// Extract capabilities, agentInfo, modes, and config options from initialize response
		const initResult = initResponse.result as
			| Record<string, unknown>
			| undefined;
		const initData: SessionInitData = {};
		if (initResult) {
			if (initResult.modes) {
				initData.modes = initResult.modes as SessionInitData["modes"];
			}
			if (initResult.configOptions) {
				initData.configOptions =
					initResult.configOptions as SessionInitData["configOptions"];
			}
			if (initResult.agentCapabilities) {
				initData.capabilities =
					initResult.agentCapabilities as SessionInitData["capabilities"];
			}
			if (initResult.agentInfo) {
				initData.agentInfo =
					initResult.agentInfo as SessionInitData["agentInfo"];
			}
		}

		const session = new Session(
			client,
			sessionId,
			agentType,
			initData,
			() => {
				this._sessions.delete(sessionId);
			},
		);
		this._sessions.set(sessionId, session);

		return { sessionId };
	}

	/**
	 * Resolve the bin entry point of an ACP adapter package.
	 * Reads from the host filesystem since kernel.readFile() does NOT see
	 * the ModuleAccessFileSystem overlay. Returns the VFS path for spawning.
	 */
	private _resolveAdapterBin(adapterPackage: string): string {
		// Check package roots first for the adapter's package.json.
		// Roots are already realpath-resolved by processSoftware.
		const vmPrefix = `/root/node_modules/${adapterPackage}`;
		let hostPkgJsonPath: string | null = null;
		for (const root of this._softwareRoots) {
			if (root.vmPath === vmPrefix) {
				hostPkgJsonPath = join(root.hostPath, "package.json");
				break;
			}
		}
		// Fall back to CWD-based node_modules.
		if (!hostPkgJsonPath) {
			hostPkgJsonPath = join(
				this._moduleAccessCwd,
				"node_modules",
				adapterPackage,
				"package.json",
			);
		}
		const pkg = JSON.parse(readFileSync(hostPkgJsonPath, "utf-8"));

		let binEntry: string | undefined;
		if (typeof pkg.bin === "string") {
			binEntry = pkg.bin;
		} else if (typeof pkg.bin === "object" && pkg.bin !== null) {
			binEntry =
				(pkg.bin as Record<string, string>)[adapterPackage] ??
				Object.values(pkg.bin)[0];
		}

		if (!binEntry) {
			throw new Error(
				`No bin entry found in ${adapterPackage}/package.json`,
			);
		}

		return `${vmPrefix}/${binEntry}`;
	}

	/**
	 * Resolve an agent config by ID. Package-provided configs take
	 * precedence over the hardcoded AGENT_CONFIGS.
	 */
	private _resolveAgentConfig(agentType: string): AgentConfig | undefined {
		return (
			this._softwareAgentConfigs.get(agentType) ??
			(AGENT_CONFIGS as Record<string, AgentConfig>)[agentType]
		);
	}

	/**
	 * Verify a session exists and is active.
	 * Throws if the session is not found.
	 */
	resumeSession(sessionId: string): { sessionId: string } {
		this._requireSession(sessionId);
		return { sessionId };
	}

	/**
	 * Gracefully destroy a session: cancel any pending work, close the client,
	 * and remove from tracking. Unlike close() which is abrupt, this attempts
	 * a graceful shutdown sequence.
	 */
	async destroySession(sessionId: string): Promise<void> {
		const session = this._sessions.get(sessionId);
		if (!session) {
			throw new Error(`Session not found: ${sessionId}`);
		}

		// Attempt graceful cancel before closing (ignore errors)
		try {
			await session.cancel();
		} catch {
			// No pending work or already closed — ignore
		}

		session.close();
	}

	// ── Flat session API (ID-based) ───────────────────────────────

	/** Send a prompt to the agent and wait for the final response. */
	async prompt(
		sessionId: string,
		text: string,
	): Promise<JsonRpcResponse> {
		return this._requireSession(sessionId).prompt(text);
	}

	/** Cancel ongoing agent work for a session. */
	async cancelSession(sessionId: string): Promise<JsonRpcResponse> {
		return this._requireSession(sessionId).cancel();
	}

	/** Kill the agent process and clear event history for a session. */
	closeSession(sessionId: string): void {
		this._requireSession(sessionId).close();
	}

	/** Returns the sequenced event history for a session. */
	getSessionEvents(
		sessionId: string,
		options?: GetEventsOptions,
	): SequencedEvent[] {
		return this._requireSession(sessionId).getSequencedEvents(options);
	}

	/** Respond to a permission request from an agent. */
	async respondPermission(
		sessionId: string,
		permissionId: string,
		reply: PermissionReply,
	): Promise<JsonRpcResponse> {
		return this._requireSession(sessionId).respondPermission(
			permissionId,
			reply,
		);
	}

	/** Set the session mode (e.g., "plan", "normal"). */
	async setSessionMode(
		sessionId: string,
		modeId: string,
	): Promise<JsonRpcResponse> {
		return this._requireSession(sessionId).setMode(modeId);
	}

	/** Returns available modes from the agent's reported capabilities. */
	getSessionModes(sessionId: string): SessionModeState | null {
		return this._requireSession(sessionId).getModes();
	}

	/** Set the model for a session. */
	async setSessionModel(
		sessionId: string,
		model: string,
	): Promise<JsonRpcResponse> {
		return this._requireSession(sessionId).setModel(model);
	}

	/** Set the thought/reasoning level for a session. */
	async setSessionThoughtLevel(
		sessionId: string,
		level: string,
	): Promise<JsonRpcResponse> {
		return this._requireSession(sessionId).setThoughtLevel(level);
	}

	/** Returns available config options for a session. */
	getSessionConfigOptions(sessionId: string): SessionConfigOption[] {
		return this._requireSession(sessionId).getConfigOptions();
	}

	/** Returns the agent's capability flags for a session. */
	getSessionCapabilities(sessionId: string): AgentCapabilities | null {
		const caps = this._requireSession(sessionId).capabilities;
		return Object.keys(caps).length > 0 ? caps : null;
	}

	/** Returns agent identity information for a session. */
	getSessionAgentInfo(sessionId: string): AgentInfo | null {
		return this._requireSession(sessionId).agentInfo;
	}

	/** Send an arbitrary JSON-RPC request to a session's agent. */
	async rawSessionSend(
		sessionId: string,
		method: string,
		params?: Record<string, unknown>,
	): Promise<JsonRpcResponse> {
		return this._requireSession(sessionId).rawSend(method, params);
	}

	/** Subscribe to session/update notifications for a session. Returns an unsubscribe function. */
	onSessionEvent(
		sessionId: string,
		handler: SessionEventHandler,
	): () => void {
		const session = this._requireSession(sessionId);
		session.onSessionEvent(handler);
		return () => {
			session.removeSessionEventHandler(handler);
		};
	}

	/** Subscribe to permission requests for a session. Returns an unsubscribe function. */
	onPermissionRequest(
		sessionId: string,
		handler: PermissionRequestHandler,
	): () => void {
		const session = this._requireSession(sessionId);
		session.onPermissionRequest(handler);
		return () => {
			session.removePermissionRequestHandler(handler);
		};
	}

	// ── Cron ────────────────────────────────────────────────────

	/** Schedule a cron job. Returns a handle with the job ID and a cancel method. */
	scheduleCron(options: CronJobOptions): CronJob {
		return this._cronManager.schedule(options);
	}

	/** List all registered cron jobs. */
	listCronJobs(): CronJobInfo[] {
		return this._cronManager.list();
	}

	/** Cancel a cron job by ID. */
	cancelCronJob(id: string): void {
		this._cronManager.cancel(id);
	}

	/** Subscribe to cron lifecycle events (fire, complete, error). */
	onCronEvent(handler: CronEventHandler): void {
		this._cronManager.onEvent(handler);
	}

	async dispose(): Promise<void> {
		// Cancel all cron jobs first
		this._cronManager.dispose();

		// Close all active sessions before disposing the kernel
		for (const session of this._sessions.values()) {
			session.close();
		}
		this._sessions.clear();

		// Kill all tracked shells
		for (const [id, entry] of this._shells) {
			entry.handle.kill();
		}
		this._shells.clear();

		// Shut down the host tools RPC server
		if (this._toolsServer) {
			await this._toolsServer.close();
			this._toolsServer = null;
		}

		return this.kernel.dispose();
	}
}
