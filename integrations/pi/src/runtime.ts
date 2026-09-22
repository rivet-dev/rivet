import { isAbsolute } from "node:path";
import {
	type AgentSession,
	type AgentSessionEvent,
	type BashOperations,
	createAgentSession,
	type CreateAgentSessionOptions,
	DefaultResourceLoader,
	getAgentDir,
	ModelRuntime,
	SessionManager,
	SettingsManager,
} from "@earendil-works/pi-coding-agent";
import type { Sandbox, SandboxProvider } from "@rivet-dev/sandbox-adapter";
import type { ActorContext } from "rivetkit";
import type { DatabaseProvider, RawAccess } from "rivetkit/db";
import { createSandboxBashOperations, createSandboxTools } from "./sandbox.js";
import {
	appendPiEntry,
	createPiSession,
	loadPiSandbox,
	loadPiSession,
	type PiSettings,
	savePiSandbox,
	savePiSettings,
	type StoredSandbox,
	toFileEntries,
} from "./storage.js";

export type PiDatabaseProvider = DatabaseProvider<RawAccess>;

/** The actor context shape every Pi helper works against. */
export type PiContext = ActorContext<
	any,
	any,
	any,
	any,
	any,
	PiDatabaseProvider,
	any,
	any
>;

export type PiSessionEventHook = (
	c: PiContext,
	event: AgentSessionEvent,
) => void | Promise<void>;

/** Pi options accepted by `pi()` on top of ordinary actor config. */
export interface PiSessionOptions
	extends Omit<CreateAgentSessionOptions, "sessionManager" | "settingsManager"> {
	/** Initial Pi settings for a brand-new session. Later changes persist per actor. */
	settings?: Partial<PiSettings>;
	/**
	 * Runs Pi's built-in file and shell tools in a sandbox. Without one, Pi has
	 * no file or shell tools and only the `customTools` passed in.
	 */
	sandbox?: SandboxProvider;
	/** Runs for every Pi session event, in addition to the `event` broadcast. */
	onSessionEvent?: PiSessionEventHook;
}

/** One open Pi session for a live actor generation. */
export interface PiSession {
	session: AgentSession;
	settingsManager: SettingsManager;
	cwd: string;
	unsubscribe: () => void;
	sandbox: ConnectedSandbox | undefined;
	/** Command execution for `executeBash`. Undefined when there is no sandbox. */
	bashOperations: BashOperations | undefined;
	/** JSON of the settings last written to SQLite, to skip no-op writes. */
	persistedSettings: string;
	/** Entries (header excluded) already written to SQLite, in Pi's append order. */
	persistedEntryCount: number;
}

/** The sandbox a session's tools run in for this actor generation. */
export interface ConnectedSandbox {
	provider: SandboxProvider;
	id: string;
	sandbox: Sandbox;
}

/** Per-actor-generation runtime state, stored on `c.vars` under `PI_RUNTIME`. */
export interface PiRuntime {
	ready?: Promise<PiSession>;
	/** Serializes SQLite entry writes so entries keep their append order. Never rejects. */
	writes: Promise<void>;
}

export const PI_RUNTIME: unique symbol = Symbol.for("@rivet-dev/pi/runtime");

export function createPiRuntime(): PiRuntime {
	return { writes: Promise.resolve() };
}

export function piRuntime(c: PiContext): PiRuntime {
	const runtime = (c.vars as Record<symbol, PiRuntime | undefined> | undefined)?.[
		PI_RUNTIME
	];
	if (!runtime) {
		throw new Error(
			"pi() runtime state is missing from actor vars; this actor was not created with pi()",
		);
	}
	return runtime;
}

/** Returns the actor's Pi session, opening it from SQLite on first use. */
export function ensurePiSession(
	c: PiContext,
	options: PiSessionOptions,
): Promise<PiSession> {
	const runtime = piRuntime(c);
	if (!runtime.ready) {
		runtime.ready = openPiSession(c, runtime, options).catch((error) => {
			runtime.ready = undefined;
			throw error;
		});
	}
	return runtime.ready;
}

/** Pi's built-in tools, which all run on the actor host. */
const PI_BUILT_IN_TOOLS = ["read", "bash", "powershell", "edit", "write", "grep", "find", "ls"];

let defaultModelRuntime: Promise<ModelRuntime> | undefined;

/** One `ModelRuntime` per process. It loads provider catalogs and credentials, which are not per actor. */
function sharedModelRuntime(): Promise<ModelRuntime> {
	defaultModelRuntime ??= ModelRuntime.create();
	return defaultModelRuntime;
}

async function openPiSession(
	c: PiContext,
	runtime: PiRuntime,
	options: PiSessionOptions,
): Promise<PiSession> {
	const { settings, onSessionEvent, sandbox: sandboxProvider, ...sessionOptions } =
		options;
	const stored = await loadPiSession(c.db);
	const connected = sandboxProvider
		? await connectSandbox(c, sandboxProvider)
		: undefined;
	const sandbox = connected?.sandbox;
	const cwd = sandbox?.cwd ?? stored?.cwd ?? sessionOptions.cwd ?? process.cwd();
	if (!isAbsolute(cwd)) {
		throw new Error(`pi() cwd must be an absolute path, received ${cwd}`);
	}
	if (stored && stored.cwd !== cwd) {
		c.log.warn({
			msg: "pi session cwd changed since it was stored",
			storedCwd: stored.cwd,
			cwd,
		});
	}

	const settingsManager = SettingsManager.inMemory(
		stored?.settings ?? settings ?? {},
	);
	const sessionManager = stored
		? SessionManager.inMemory(cwd, undefined, toFileEntries(stored))
		: SessionManager.inMemory(cwd);
	const modelRuntime = sessionOptions.modelRuntime ?? (await sharedModelRuntime());
	const resourceLoader =
		sessionOptions.resourceLoader ??
		(await isolatedResourceLoader(cwd, sessionOptions.agentDir, settingsManager));

	const { session, modelFallbackMessage } = await createAgentSession({
		...sessionOptions,
		cwd,
		modelRuntime,
		settingsManager,
		sessionManager,
		resourceLoader,
		customTools: sandbox
			? [...(sessionOptions.customTools ?? []), ...createSandboxTools(sandbox)]
			: sessionOptions.customTools,
		// Pi's built-in tools run on the actor host. With a sandbox, the sandbox
		// versions replace them and PowerShell is removed. Without one, all of
		// them are removed so the agent cannot reach the actor host.
		excludeTools: [
			...new Set([
				...(sessionOptions.excludeTools ?? []),
				...(sandbox ? ["powershell"] : PI_BUILT_IN_TOOLS),
			]),
		],
	});
	if (modelFallbackMessage) {
		c.log.warn({ msg: "pi model fallback", detail: modelFallbackMessage });
	}

	const handle: PiSession = {
		session,
		settingsManager,
		cwd,
		unsubscribe: () => {},
		sandbox: connected,
		bashOperations: sandbox ? createSandboxBashOperations(sandbox) : undefined,
		persistedSettings: JSON.stringify(settingsManager.getGlobalSettings()),
		persistedEntryCount: stored?.entries.length ?? 0,
	};

	if (!stored) {
		const header = sessionManager.getHeader();
		if (!header) {
			throw new Error("Pi did not create a session header");
		}
		await createPiSession(c.db, {
			header,
			cwd,
			settings: JSON.parse(handle.persistedSettings) as PiSettings,
		});
	}
	await flushPiEntries(c, runtime, handle);
	handle.unsubscribe = session.subscribe((event) => {
		broadcastSessionEvent(c, event);
		// Pi appends entries from inside its agent loop without a dedicated
		// event, so new entries are found by diffing the entry list.
		if (!isStreamingDelta(event)) {
			flushPiEntries(c, runtime, handle).catch((error: unknown) => {
				c.log.error({
					msg: "pi session entry write failed, retrying on the next flush",
					error,
				});
			});
		}
		if (onSessionEvent) {
			c.waitUntil(
				Promise.resolve()
					.then(() => onSessionEvent(c, event))
					.catch((error: unknown) => {
						c.log.error({ msg: "pi session event hook failed", error });
					}),
			);
		}
	});

	c.log.info({
		msg: "pi session opened",
		sessionId: session.sessionId,
		restored: stored !== undefined,
		entryCount: sessionManager.getEntries().length,
		messageCount: session.messages.length,
	});
	return handle;
}

/**
 * Connects to the actor's sandbox, creating one when none is stored or the
 * provider reports the stored one no longer exists. A new sandbox id is saved
 * as soon as `create` returns, so a failure later in the start reuses it. Any
 * other connect failure is thrown, so a temporary outage never replaces a
 * sandbox.
 */
async function connectSandbox(
	c: PiContext,
	provider: SandboxProvider,
): Promise<ConnectedSandbox> {
	const existing = await loadPiSandbox(c.db);
	if (existing && existing.provider !== provider.name) {
		throw new Error(
			`pi sandbox was created by provider ${existing.provider}, but the actor now uses ${provider.name}`,
		);
	}
	if (existing) {
		const sandbox = await provider.connect(c, existing.id);
		if (sandbox) return { provider, id: existing.id, sandbox };
		c.log.warn({
			msg: "pi sandbox no longer exists, creating a new one; files from the previous sandbox are lost",
			provider: provider.name,
			sandboxId: existing.id,
		});
	}
	const id = await provider.create(c);
	await savePiSandbox(c.db, { provider: provider.name, id });
	const sandbox = await provider.connect(c, id);
	if (!sandbox) {
		throw new Error(`pi sandbox ${provider.name}/${id} was not found right after it was created`);
	}
	return { provider, id, sandbox };
}

/**
 * Pi's resource discovery reads the actor host's filesystem and loads host
 * code as extensions. Extensions, skills, prompt templates, context files, and
 * themes stay off unless the developer passes a loader.
 */
async function isolatedResourceLoader(
	cwd: string,
	agentDir: string | undefined,
	settingsManager: SettingsManager,
): Promise<DefaultResourceLoader> {
	const loader = new DefaultResourceLoader({
		cwd,
		agentDir: agentDir ?? getAgentDir(),
		settingsManager,
		noExtensions: true,
		noSkills: true,
		noPromptTemplates: true,
		noContextFiles: true,
		noThemes: true,
	});
	await loader.reload();
	return loader;
}

/** Token and output deltas never append entries, so they skip the entry diff. */
function isStreamingDelta(event: AgentSessionEvent): boolean {
	return (
		event.type === "message_update" ||
		event.type === "tool_execution_update" ||
		event.type === "bash_execution_update"
	);
}

/**
 * Writes every entry Pi appended since the last successful write, in append
 * order. The count only advances after an insert succeeds, so a failed write
 * is retried by the next flush. Returns the write so actions can await it.
 */
function flushPiEntries(
	c: PiContext,
	runtime: PiRuntime,
	handle: PiSession,
): Promise<void> {
	const write = runtime.writes.then(async () => {
		const entries = handle.session.sessionManager.getEntries();
		while (handle.persistedEntryCount < entries.length) {
			await appendPiEntry(c.db, entries[handle.persistedEntryCount]!);
			handle.persistedEntryCount += 1;
		}
	});
	runtime.writes = write.catch(() => {});
	c.waitUntil(runtime.writes);
	return write;
}

function broadcastSessionEvent(c: PiContext, event: AgentSessionEvent): void {
	try {
		c.broadcast("event", event);
	} catch (error) {
		if (isActorStoppingError(error)) return;
		c.log.error({ msg: "failed to broadcast pi session event", error });
	}
}

function isActorStoppingError(error: unknown): boolean {
	if (!error || typeof error !== "object") return false;
	const candidate = error as { group?: unknown; code?: unknown };
	return candidate.group === "actor" && candidate.code === "stopping";
}

/**
 * Queues entries appended since the last flush and writes Pi's mutable settings
 * when they changed. Called after every action and on shutdown.
 */
export async function persistPiState(
	c: PiContext,
	handle: PiSession,
): Promise<void> {
	await flushPiEntries(c, piRuntime(c), handle);
	const settings = handle.settingsManager.getGlobalSettings();
	const encoded = JSON.stringify(settings);
	if (encoded === handle.persistedSettings) return;
	await savePiSettings(c.db, settings);
	handle.persistedSettings = encoded;
}

/**
 * Stops the Pi session for this actor generation: aborts any run, lets
 * extensions shut down, and flushes settings and entries. Then suspends the
 * sandbox on sleep, or destroys it on destroy.
 */
export async function closePiSession(
	c: PiContext,
	options: PiSessionOptions,
	reason: "sleep" | "destroy",
): Promise<void> {
	const runtime = piRuntime(c);
	const ready = runtime.ready;
	runtime.ready = undefined;
	const errors: unknown[] = [];
	let handle: PiSession | undefined;
	try {
		handle = await ready;
	} catch {
		// Opening failed and was already reported to the caller.
	}

	if (handle) {
		const open = handle;
		await attempt(errors, () => open.session.abort());
		await attempt(errors, async () => {
			if (open.session.hasExtensionHandlers("session_shutdown")) {
				await open.session.extensionRunner.emit({
					type: "session_shutdown",
					reason: "quit",
				});
			}
		});
		await attempt(errors, () => persistPiState(c, open));
		open.unsubscribe();
		open.session.dispose();
		c.log.info({ msg: "pi session closed", sessionId: open.session.sessionId });
	}
	await runtime.writes;

	const provider = options.sandbox;
	if (provider) {
		await attempt(errors, async () => {
			if (reason === "sleep") {
				// Only a sandbox connected in this generation can be running.
				if (handle?.sandbox && provider.suspend) {
					await provider.suspend(c, handle.sandbox.id);
				}
				return;
			}
			const sandbox = handle?.sandbox ?? (await storedSandbox(c, provider));
			if (sandbox && provider.destroy) {
				await provider.destroy(c, sandbox.id);
			}
		});
	}

	if (errors.length === 1) throw errors[0];
	if (errors.length > 1) {
		throw new AggregateError(errors, "pi session shutdown failed");
	}
}

/** The stored sandbox, when it belongs to `provider`. */
async function storedSandbox(
	c: PiContext,
	provider: SandboxProvider,
): Promise<StoredSandbox | undefined> {
	const sandbox = await loadPiSandbox(c.db);
	return sandbox?.provider === provider.name ? sandbox : undefined;
}

async function attempt(
	errors: unknown[],
	operation: () => void | Promise<void>,
): Promise<void> {
	try {
		await operation();
	} catch (error) {
		errors.push(error);
	}
}
