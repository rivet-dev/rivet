import { isAbsolute } from "node:path";
import {
	type AgentSession,
	type AgentSessionEvent,
	createAgentSession,
	type CreateAgentSessionOptions,
	ModelRuntime,
	SessionManager,
	SettingsManager,
} from "@earendil-works/pi-coding-agent";
import type { ActorContext } from "rivetkit";
import type { DatabaseProvider, RawAccess } from "rivetkit/db";
import {
	appendPiEntry,
	createPiSession,
	loadPiSession,
	type PiSettings,
	savePiSettings,
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

/** Pi options accepted by `pi()` on top of ordinary actor config. */
export interface PiSessionOptions
	extends Omit<CreateAgentSessionOptions, "sessionManager" | "settingsManager"> {
	/** Initial Pi settings for a brand-new session. Later changes persist per actor. */
	settings?: Partial<PiSettings>;
}

/** One open Pi session for a live actor generation. */
export interface PiSession {
	session: AgentSession;
	settingsManager: SettingsManager;
	cwd: string;
	/** JSON of the settings last written to SQLite, to skip no-op writes. */
	persistedSettings: string;
	/** Entries (header excluded) already written to SQLite, in Pi's append order. */
	persistedEntryCount: number;
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
	const { settings, ...sessionOptions } = options;
	const stored = await loadPiSession(c.db);
	const cwd = stored?.cwd ?? sessionOptions.cwd ?? process.cwd();
	if (!isAbsolute(cwd)) {
		throw new Error(`pi() cwd must be an absolute path, received ${cwd}`);
	}

	const settingsManager = SettingsManager.inMemory(
		stored?.settings ?? settings ?? {},
	);
	const sessionManager = stored
		? SessionManager.inMemory(cwd, undefined, toFileEntries(stored))
		: SessionManager.inMemory(cwd);
	const modelRuntime = sessionOptions.modelRuntime ?? (await sharedModelRuntime());

	const { session, modelFallbackMessage } = await createAgentSession({
		...sessionOptions,
		cwd,
		modelRuntime,
		settingsManager,
		sessionManager,
	});
	if (modelFallbackMessage) {
		c.log.warn({ msg: "pi model fallback", detail: modelFallbackMessage });
	}

	const handle: PiSession = {
		session,
		settingsManager,
		cwd,
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
	session.subscribe((event) => {
		broadcastSessionEvent(c, event);
		if (!isStreamingDelta(event)) {
			flushPiEntries(c, runtime, handle).catch((error: unknown) => {
				c.log.error({
					msg: "pi session entry write failed, retrying on the next flush",
					error,
				});
			});
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
 * extensions shut down, flushes settings, and waits for queued entry writes.
 */
export async function closePiSession(c: PiContext): Promise<void> {
	const runtime = piRuntime(c);
	const ready = runtime.ready;
	runtime.ready = undefined;
	if (!ready) return;
	let handle: PiSession;
	try {
		handle = await ready;
	} catch {
		return;
	}

	const errors: unknown[] = [];
	await attempt(errors, () => handle.session.abort());
	await attempt(errors, async () => {
		if (handle.session.hasExtensionHandlers("session_shutdown")) {
			await handle.session.extensionRunner.emit({
				type: "session_shutdown",
				reason: "quit",
			});
		}
	});
	await attempt(errors, () => persistPiState(c, handle));
	handle.session.dispose();

	c.log.info({ msg: "pi session closed", sessionId: handle.session.sessionId });
	if (errors.length === 1) throw errors[0];
	if (errors.length > 1) {
		throw new AggregateError(errors, "pi session shutdown failed");
	}
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
