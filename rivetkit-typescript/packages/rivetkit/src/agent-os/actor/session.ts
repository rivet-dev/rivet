import crypto from "node:crypto";
import type {
	AgentOs,
	AgentType,
	PendingPermissionRequest,
	PermissionResponseResult,
	SessionConfig,
	SessionInfo,
	SessionStreamEntry,
} from "@rivet-dev/agent-os-core";
import type { AgentOsActorConfig } from "../config";
import type {
	AgentOsActionContext,
	CreateSessionOptions,
	PersistedSessionEvent,
	PersistedSessionRecord,
	PromptResult,
	SessionRecord,
} from "../types";
import { ensureVm, runHook, syncPreventSleep, truncateForLog } from "./index";

// The actor keeps a pull contract — `getEvents` / `getSequencedEvents`
// actions — by serving reads from the persisted event ledger this file
// maintains via its own `onSessionEvent` subscription (agent-os has no
// pull API; remote pollers need one that survives reconnects and sleep).
interface GetEventsOptions {
	/** Only return events with `sequenceNumber >= since`. */
	since?: number;
}

interface SequencedEvent {
	sequenceNumber: number;
	entry: SessionStreamEntry;
}

/**
 * Pick the adapter-supplied ACP permission option matching a decision kind.
 * 0.2.8's `respondPermission` requires an EXACT `optionId` from the
 * request's own options — hooks decide with a kind
 * (`allow_once`/`allow_always`/`reject_once`/`reject_always`) and this
 * resolves it, falling back to the canonical id when the adapter's options
 * are unavailable (e.g. responding after an actor wake).
 */
export function pickPermissionOptionId(
	options: PendingPermissionRequest["options"] | undefined,
	kind: "allow_once" | "allow_always" | "reject_once" | "reject_always",
): string {
	const matched = options?.find(
		(option) => option.kind === kind || option.optionId === kind,
	);
	if (matched && typeof matched.optionId === "string") {
		return matched.optionId;
	}
	return kind;
}

// Helper to verify a session exists in the VM. Throws via AgentOs if not found.
function assertSessionExists<TConnParams>(
	c: AgentOsActionContext<TConnParams>,
	sessionId: string,
): void {
	if (!c.vars.sessions.has(sessionId)) {
		throw new Error(`session not found: ${sessionId}`);
	}
}

/** Concatenated text blocks of the final agent message. */
function promptMessageText(
	message: { content?: Array<Record<string, unknown>> } | null,
): string {
	if (!message?.content) return "";
	let text = "";
	for (const block of message.content) {
		if (block.type === "text" && typeof block.text === "string") {
			text += block.text;
		}
	}
	return text;
}

// Build a SessionRecord from the AgentOs durable-session API.
async function toSessionRecord(
	agentOs: AgentOs,
	sessionId: string,
	agentType: string,
): Promise<SessionRecord> {
	// Snapshot capabilities/agentInfo into plain JSON objects INSIDE the
	// action body. The core returns live (proxy/NAPI-backed) objects whose
	// late property reads during response encoding can fail against a
	// wedged sidecar, surfacing as an opaque `internal_error` on an action
	// whose body fully succeeded (live signature 2026-07: createSession
	// works via raw probes while the wrapper action fails).
	const capabilities =
		(await agentOs.getSessionCapabilities({ sessionId })) ?? {};
	const agentInfo = await agentOs.getSessionAgentInfo({ sessionId });
	return JSON.parse(
		JSON.stringify({
			sessionId,
			agentType,
			capabilities,
			agentInfo,
		}),
	) as SessionRecord;
}

// --- Session persistence helpers ---

// Persist a session record to SQLite when it is created.
async function persistSession<TConnParams>(
	c: AgentOsActionContext<TConnParams>,
	agentOs: AgentOs,
	sessionId: string,
	agentType: string,
): Promise<void> {
	const now = Date.now();
	const capabilities =
		(await agentOs.getSessionCapabilities({ sessionId })) ?? {};
	const agentInfo = await agentOs.getSessionAgentInfo({ sessionId });
	await c.db.execute(
		`INSERT OR REPLACE INTO agent_os_sessions (session_id, agent_type, capabilities, agent_info, created_at)
		 VALUES (?, ?, ?, ?, ?)`,
		sessionId,
		agentType,
		JSON.stringify(capabilities),
		agentInfo ? JSON.stringify(agentInfo) : null,
		now,
	);
}

// Persist a session stream entry to SQLite with an auto-incrementing
// sequence number (the actor's own ledger seq, independent of the core's
// durable `sequence` — ephemeral chunk entries have none).
async function persistSessionEvent<TConnParams>(
	c: AgentOsActionContext<TConnParams>,
	sessionId: string,
	entry: SessionStreamEntry,
): Promise<void> {
	const now = Date.now();

	// Compute next sequence number for this session.
	const rows: { max_seq: number | null }[] = await c.db.execute(
		`SELECT MAX(seq) as max_seq FROM agent_os_session_events WHERE session_id = ?`,
		sessionId,
	);
	const nextSeq = (rows[0]?.max_seq ?? -1) + 1;

	await c.db.execute(
		`INSERT INTO agent_os_session_events (session_id, seq, event, created_at)
		 VALUES (?, ?, ?, ?)`,
		sessionId,
		nextSeq,
		JSON.stringify(entry),
		now,
	);
}

// Read persisted events for a session with seq >= since, oldest first.
async function readPersistedEventsSince<TConnParams>(
	c: AgentOsActionContext<TConnParams>,
	sessionId: string,
	since: number,
): Promise<SequencedEvent[]> {
	const rows: { seq: number; event: string }[] = await c.db.execute(
		`SELECT seq, event
		 FROM agent_os_session_events
		 WHERE session_id = ? AND seq >= ?
		 ORDER BY seq ASC`,
		sessionId,
		since,
	);
	return rows.map((row) => ({
		sequenceNumber: row.seq,
		entry: JSON.parse(row.event) as SessionStreamEntry,
	}));
}

// Remove a session and its events from SQLite.
async function deletePersistedSession<TConnParams>(
	c: AgentOsActionContext<TConnParams>,
	sessionId: string,
): Promise<void> {
	await c.db.execute(
		`DELETE FROM agent_os_session_events WHERE session_id = ?`,
		sessionId,
	);
	await c.db.execute(
		`DELETE FROM agent_os_sessions WHERE session_id = ?`,
		sessionId,
	);
}

/**
 * Rebuild the durable-stream high-water mark from the SQLite ledger.
 * `sessionEventSequences` lives in ephemeral vars and is empty after sleep;
 * without this, 0.2.8 live redelivery re-persists and re-fires hooks.
 */
async function hydrateDurableSequenceHighWater<TConnParams>(
	c: AgentOsActionContext<TConnParams>,
	sessionId: string,
): Promise<void> {
	const events = await readPersistedEventsSince(c, sessionId, 0);
	let max: number | undefined;
	for (const { entry } of events) {
		if (
			entry?.durability === "durable" &&
			typeof entry.sequence === "number"
		) {
			if (max === undefined || entry.sequence > max) max = entry.sequence;
		}
	}
	if (max !== undefined) {
		c.vars.sessionEventSequences.set(sessionId, max);
	}
}

/** Best-effort delete when createSession fails after openSession. */
async function bestEffortDeleteSession(
	agentOs: AgentOs,
	sessionId: string,
	log: { warn: (fields: Record<string, unknown>) => void },
): Promise<void> {
	try {
		await agentOs.deleteSession({ sessionId });
	} catch (cleanupErr) {
		log.warn({
			msg: "agent-os: failed to delete orphaned session after create failure",
			sessionId,
			err: truncateForLog(
				cleanupErr instanceof Error
					? cleanupErr.message
					: String(cleanupErr),
			),
		});
	}
}

// Subscribe to a session's stream via the per-session AgentOs overload,
// broadcasting entries and running user-provided hooks.
export function subscribeToSession<TConnParams>(
	c: AgentOsActionContext<TConnParams>,
	agentOs: AgentOs,
	sessionId: string,
	parsedConfig: AgentOsActorConfig<TConnParams>,
): void {
	// Idempotent per session: `resumeSession` re-opens an existing durable
	// session on the same VM — a second handler would double-broadcast and
	// double-persist every event.
	if (c.vars.sessions.has(sessionId)) {
		return;
	}

	agentOs.onSessionEvent(sessionId, (entry: SessionStreamEntry) => {
		// Always fan the event out, regardless of `c.abortSignal.aborted`.
		// `c` here is captured from the `createSession` action's context.
		// Action-context abortSignals can fire *call-scoped* (after the
		// action returns / when the initiating WS connection drops) while
		// the ACTOR is still very much alive and the agent session is still
		// streaming thoughts. An early return here silently dropped every
		// subsequent `agent_thought_chunk` and (worse) the user's
		// `onSessionEvent` hook — meaning the DAG kept growing (tool calls
		// succeeded via direct actor RPCs) but the live thinking transcript
		// never reached the UI.
		//
		// Both downstream paths handle shutdown-time races themselves:
		// `c.broadcast` is best-effort, and `persistSessionEvent`'s
		// `.catch` below demotes shutdown-time DB errors to a log line.

		// Durable entries carry the core's sequence; 0.2.8 documents that
		// duplicates can occur on live delivery — drop exact re-deliveries
		// so broadcasts and the ledger stay append-once.
		if (entry.durability === "durable") {
			const lastSeq = c.vars.sessionEventSequences.get(sessionId);
			if (lastSeq !== undefined && entry.sequence <= lastSeq) {
				return;
			}
			c.vars.sessionEventSequences.set(sessionId, entry.sequence);
		}

		// Permission traffic rides the same stream; surface requests on the
		// dedicated permission channel (broadcast + hook) so actors can
		// answer them via `respondPermission`.
		if (entry.type === "permission_request") {
			const request: PendingPermissionRequest = {
				requestId: entry.requestId,
				options: entry.options,
				toolCall: entry.toolCall,
				...(entry._meta !== undefined ? { _meta: entry._meta } : {}),
			};

			c.broadcast(
				"permissionRequest",
				JSON.parse(JSON.stringify({ sessionId, request })),
			);

			if (parsedConfig.onPermissionRequest) {
				runHook(c, "onPermissionRequest", () =>
					parsedConfig.onPermissionRequest?.(c, sessionId, request),
				);
			}
			return;
		}

		c.broadcast(
			"sessionEvent",
			JSON.parse(JSON.stringify({ sessionId, entry })),
		);

		// Persist to the ledger for reconnect replay (`getSequencedEvents`).
		persistSessionEvent(c, sessionId, entry).catch((error) =>
			c.log.error({
				msg: "agent-os failed to persist session event",
				sessionId,
				error,
			}),
		);

		if (parsedConfig.onSessionEvent) {
			runHook(c, "onSessionEvent", () =>
				parsedConfig.onSessionEvent?.(c, sessionId, entry),
			);
		}
	});

	c.vars.sessions.add(sessionId);
}

// Build session management actions for the actor factory.
export function buildSessionActions<TConnParams>(
	config: AgentOsActorConfig<TConnParams>,
) {
	return {
		createSession: async (
			c: AgentOsActionContext<TConnParams>,
			agentType: AgentType,
			options?: CreateSessionOptions,
		): Promise<SessionRecord> => {
			const agentOs = await ensureVm(c, config);
			// `openSession` returns void — the actor owns the id.
			const sessionId = crypto.randomUUID();
			try {
				await agentOs.openSession({
					...options,
					sessionId,
					agent: agentType,
					// 0.2.8 defaults to `allow_all`, which auto-answers
					// adapter permission requests WITHOUT surfacing them —
					// bypassing the actors' permission hooks. When a hook is
					// configured, FORCE ask and ignore any caller override
					// (browser-reachable vmV2 can otherwise pass allow_all).
					permissionPolicy: config.onPermissionRequest
						? "ask"
						: (options?.permissionPolicy ?? "allow_all"),
				});
			} catch (err) {
				// Surface the REAL failure (bootVm precedent): the actor
				// runtime wraps this throw as an opaque `internal_error`,
				// which made live createSession failures undiagnosable.
				c.log.error({
					msg: "agent-os: createSession failed",
					agentType,
					err: truncateForLog(
						err instanceof Error ? err.message : String(err),
					),
					stack: truncateForLog(
						err instanceof Error ? err.stack : undefined,
					),
				});
				throw err;
			}
			try {
				await hydrateDurableSequenceHighWater(c, sessionId);
				subscribeToSession(c, agentOs, sessionId, config);

				// Persist session metadata to SQLite for sleep/wake recovery.
				await persistSession(c, agentOs, sessionId, agentType);

				c.log.info({
					msg: "agent-os session created",
					sessionId,
					agentType,
				});
				return await toSessionRecord(agentOs, sessionId, agentType);
			} catch (err) {
				// Same visibility for post-create failures (subscribe/persist/
				// record build) — otherwise they surface as opaque internal_error
				// while the created session silently leaks.
				c.log.error({
					msg: "agent-os: createSession post-create failed",
					sessionId,
					agentType,
					err: truncateForLog(
						err instanceof Error ? err.message : String(err),
					),
					stack: truncateForLog(
						err instanceof Error ? err.stack : undefined,
					),
				});
				await bestEffortDeleteSession(agentOs, sessionId, c.log);
				c.vars.sessions.delete(sessionId);
				c.vars.activeSessionIds.delete(sessionId);
				c.vars.sessionEventSequences.delete(sessionId);
				throw err;
			}
		},

		listSessions: async (
			c: AgentOsActionContext<TConnParams>,
		): Promise<SessionInfo[]> => {
			const agentOs = await ensureVm(c, config);
			// Drain all pages.
			const sessions: SessionInfo[] = [];
			let cursor: string | undefined;
			do {
				const page = await agentOs.listSessions(
					cursor ? { cursor } : undefined,
				);
				sessions.push(...page.sessions);
				cursor = page.nextCursor ?? undefined;
			} while (cursor);
			return JSON.parse(JSON.stringify(sessions)) as SessionInfo[];
		},

		getSession: async (
			c: AgentOsActionContext<TConnParams>,
			sessionId: string,
		): Promise<SessionRecord> => {
			assertSessionExists(c, sessionId);
			const agentOs = await ensureVm(c, config);
			let agentType: string;
			try {
				const info = await agentOs.getSession({ sessionId });
				agentType = info.agent;
			} catch {
				throw new Error(`session not found: ${sessionId}`);
			}
			return toSessionRecord(agentOs, sessionId, agentType);
		},

		destroySession: async (
			c: AgentOsActionContext<TConnParams>,
			sessionId: string,
		): Promise<void> => {
			const agentOs = await ensureVm(c, config);
			await agentOs.deleteSession({ sessionId });
			c.vars.sessions.delete(sessionId);
			c.vars.activeSessionIds.delete(sessionId);
			c.vars.sessionEventSequences.delete(sessionId);
			syncPreventSleep(c);

			// Clean up persisted session and events from SQLite.
			await deletePersistedSession(c, sessionId);

			c.log.info({ msg: "agent-os session destroyed", sessionId });
		},

		resumeSession: async (
			c: AgentOsActionContext<TConnParams>,
			sessionId: string,
		): Promise<{ sessionId: string }> => {
			const agentOs = await ensureVm(c, config);
			// Recover the agent type from the persisted session row (written
			// on createSession) — `openSession` needs it, and re-opening an
			// existing durable id restores its adapter.
			const rows: { agent_type: string }[] = await c.db.execute(
				`SELECT agent_type FROM agent_os_sessions WHERE session_id = ?`,
				sessionId,
			);
			const agentType: AgentType | undefined = rows[0]?.agent_type;
			if (!agentType) {
				throw new Error(
					`cannot resume session ${sessionId}: no recorded agent type`,
				);
			}
			await agentOs.openSession({
				sessionId,
				agent: agentType,
				permissionPolicy: config.onPermissionRequest
					? "ask"
					: "allow_all",
			});
			await hydrateDurableSequenceHighWater(c, sessionId);
			subscribeToSession(c, agentOs, sessionId, config);
			return { sessionId };
		},

		closeSession: async (
			c: AgentOsActionContext<TConnParams>,
			sessionId: string,
		): Promise<void> => {
			const agentOs = await ensureVm(c, config);
			await agentOs.unloadSession({ sessionId });
			c.vars.sessions.delete(sessionId);
			c.vars.activeSessionIds.delete(sessionId);
			c.vars.sessionEventSequences.delete(sessionId);
			syncPreventSleep(c);

			// Clean up persisted session and events from SQLite.
			await deletePersistedSession(c, sessionId);

			c.log.info({ msg: "agent-os session closed", sessionId });
		},
	};
}

// Build prompt, cancel, and permission actions for the actor factory.
export function buildPromptActions<TConnParams>(
	config: AgentOsActorConfig<TConnParams>,
) {
	/**
	 * 0.2.8 streams only flattened SessionStreamEntry (no method tails).
	 * Product actors still key burst drain / watchdog / turnCount off
	 * legacy `session/completed|aborted` method events — synthesize those
	 * when the blocking prompt API resolves so turn-end side effects run.
	 */
	const emitPromptTerminal = (
		c: AgentOsActionContext<TConnParams>,
		sessionId: string,
		method: "session/completed" | "session/aborted",
		params?: Record<string, unknown>,
	) => {
		if (!config.onSessionEvent) return;
		const terminalEvent = { method, params: params ?? {} };
		runHook(c, "onSessionEvent", () =>
			config.onSessionEvent?.(c, sessionId, terminalEvent),
		);
	};

	return {
		sendPrompt: async (
			c: AgentOsActionContext<TConnParams>,
			sessionId: string,
			text: string,
		): Promise<PromptResult> => {
			if (c.aborted) {
				throw new Error(
					"actor is shutting down, cannot start new prompt",
				);
			}

			assertSessionExists(c, sessionId);
			const agentOs = c.vars.agentOs;
			if (!agentOs) {
				throw new Error("VM not initialized");
			}

			c.vars.activeSessionIds.add(sessionId);
			syncPreventSleep(c);
			c.log.info({ msg: "agent-os prompt turn started", sessionId });

			const start = Date.now();
			let promptResult: PromptResult | undefined;
			let promptFailed = false;
			try {
				const result = await agentOs.prompt({
					sessionId,
					content: [{ type: "text", text }],
				});
				promptResult = JSON.parse(
					JSON.stringify({
						sessionId: result.sessionId,
						stopReason: result.stopReason,
						message: result.message,
						text: promptMessageText(result.message),
					}),
				) as PromptResult;
				return promptResult;
			} catch (err) {
				promptFailed = true;
				throw err;
			} finally {
				c.vars.activeSessionIds.delete(sessionId);
				syncPreventSleep(c);
				c.log.info({
					msg: "agent-os prompt turn ended",
					sessionId,
					durationMs: Date.now() - start,
				});
				// Emit after activeSessionIds clear so sleep can proceed;
				// hooks are fire-and-forget via runHook.
				if (promptFailed) {
					emitPromptTerminal(c, sessionId, "session/aborted", {
						reason: "error",
					});
				} else if (promptResult) {
					emitPromptTerminal(c, sessionId, "session/completed", {
						stopReason: promptResult.stopReason,
					});
				}
			}
		},

		cancelPrompt: async (
			c: AgentOsActionContext<TConnParams>,
			sessionId: string,
		): Promise<{ status: "cancelled" | "no_active_prompt" }> => {
			assertSessionExists(c, sessionId);
			const agentOs = c.vars.agentOs;
			if (!agentOs) {
				throw new Error("VM not initialized");
			}
			const result = await agentOs.cancelPrompt({ sessionId });
			if (result.status === "cancelled") {
				emitPromptTerminal(c, sessionId, "session/aborted", {
					reason: "cancelled",
				});
			}
			return result;
		},

		respondPermission: async (
			c: AgentOsActionContext<TConnParams>,
			sessionId: string,
			requestId: string,
			optionId: string,
		): Promise<PermissionResponseResult> => {
			assertSessionExists(c, sessionId);
			const agentOs = c.vars.agentOs;
			if (!agentOs) {
				throw new Error("VM not initialized");
			}
			const result = await agentOs.respondPermission({
				sessionId,
				requestId,
				optionId,
			});
			return JSON.parse(
				JSON.stringify(result),
			) as PermissionResponseResult;
		},
	};
}

// Build session configuration proxy actions for the actor factory.
export function buildConfigActions<TConnParams>(
	_config: AgentOsActorConfig<TConnParams>,
) {
	return {
		getSessionConfig: async (
			c: AgentOsActionContext<TConnParams>,
			sessionId: string,
		): Promise<SessionConfig> => {
			assertSessionExists(c, sessionId);
			const agentOs = c.vars.agentOs;
			if (!agentOs) {
				throw new Error("VM not initialized");
			}
			const sessionConfig = await agentOs.getSessionConfig({ sessionId });
			return JSON.parse(JSON.stringify(sessionConfig)) as SessionConfig;
		},

		setSessionConfigOption: async (
			c: AgentOsActionContext<TConnParams>,
			sessionId: string,
			configId: string,
			value: string | boolean,
		): Promise<SessionConfig> => {
			assertSessionExists(c, sessionId);
			const agentOs = c.vars.agentOs;
			if (!agentOs) {
				throw new Error("VM not initialized");
			}
			const updated = await agentOs.setSessionConfigOption({
				sessionId,
				configId,
				value,
			});
			return JSON.parse(JSON.stringify(updated)) as SessionConfig;
		},

		getEvents: async (
			c: AgentOsActionContext<TConnParams>,
			sessionId: string,
			options?: GetEventsOptions,
		): Promise<SessionStreamEntry[]> => {
			assertSessionExists(c, sessionId);
			const events = await readPersistedEventsSince(
				c,
				sessionId,
				options?.since ?? 0,
			);
			return events.map((e) => e.entry);
		},

		getSequencedEvents: async (
			c: AgentOsActionContext<TConnParams>,
			sessionId: string,
			options?: GetEventsOptions,
		): Promise<SequencedEvent[]> => {
			assertSessionExists(c, sessionId);
			return readPersistedEventsSince(c, sessionId, options?.since ?? 0);
		},
	};
}

// Build actions for querying persisted session data from SQLite.
// These work without a running VM and return data from prior sessions
// that survived sleep/wake cycles.
export function buildSessionPersistenceActions<TConnParams>(
	_config: AgentOsActorConfig<TConnParams>,
) {
	return {
		listPersistedSessions: async (
			c: AgentOsActionContext<TConnParams>,
		): Promise<PersistedSessionRecord[]> => {
			const rows: {
				session_id: string;
				agent_type: string;
				capabilities: string;
				agent_info: string | null;
				created_at: number;
			}[] = await c.db.execute(
				`SELECT session_id, agent_type, capabilities, agent_info, created_at
				 FROM agent_os_sessions
				 ORDER BY created_at ASC`,
			);

			return rows.map((row) => ({
				sessionId: row.session_id,
				agentType: row.agent_type,
				capabilities: JSON.parse(row.capabilities),
				agentInfo: row.agent_info ? JSON.parse(row.agent_info) : null,
				createdAt: row.created_at,
			}));
		},

		getSessionEvents: async (
			c: AgentOsActionContext<TConnParams>,
			sessionId: string,
		): Promise<PersistedSessionEvent[]> => {
			const rows: {
				session_id: string;
				seq: number;
				event: string;
				created_at: number;
			}[] = await c.db.execute(
				`SELECT session_id, seq, event, created_at
				 FROM agent_os_session_events
				 WHERE session_id = ?
				 ORDER BY seq ASC`,
				sessionId,
			);

			return rows.map((row) => ({
				sessionId: row.session_id,
				seq: row.seq,
				entry: JSON.parse(row.event),
				createdAt: row.created_at,
			}));
		},
	};
}
