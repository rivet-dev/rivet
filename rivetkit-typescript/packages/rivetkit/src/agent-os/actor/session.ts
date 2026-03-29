import type {
	AgentOs,
	AgentType,
	CreateSessionOptions,
	GetEventsOptions,
	JsonRpcNotification,
	JsonRpcResponse,
	PermissionReply,
	SequencedEvent,
	SessionConfigOption,
	SessionInfo,
	SessionModeState,
} from "@rivet-dev/agent-os-core";
import type { AgentOsActorConfig } from "../config";
import type {
	AgentOsActionContext,
	PersistedSessionEvent,
	PersistedSessionRecord,
	SessionRecord,
} from "../types";
import { ensureVm, runHook, syncPreventSleep } from "./index";

// Helper to verify a session exists in the VM. Throws via AgentOs if not found.
function assertSessionExists<TConnParams>(
	c: AgentOsActionContext<TConnParams>,
	sessionId: string,
): void {
	if (!c.vars.sessions.has(sessionId)) {
		throw new Error(`session not found: ${sessionId}`);
	}
}

// Build a SessionRecord from AgentOs flat API.
function toSessionRecord(
	agentOs: AgentOs,
	sessionId: string,
	agentType: string,
): SessionRecord {
	return {
		sessionId,
		agentType,
		capabilities: agentOs.getSessionCapabilities(sessionId) ?? {},
		agentInfo: agentOs.getSessionAgentInfo(sessionId),
	};
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
	const capabilities = agentOs.getSessionCapabilities(sessionId) ?? {};
	const agentInfo = agentOs.getSessionAgentInfo(sessionId);
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

// Persist a session event to SQLite with an auto-incrementing sequence number.
async function persistSessionEvent<TConnParams>(
	c: AgentOsActionContext<TConnParams>,
	sessionId: string,
	event: JsonRpcNotification,
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
		JSON.stringify(event),
		now,
	);
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

// Subscribe to a session's event and permission streams via the flat AgentOs API,
// broadcasting events and running user-provided hooks.
export function subscribeToSession<TConnParams>(
	c: AgentOsActionContext<TConnParams>,
	agentOs: AgentOs,
	sessionId: string,
	parsedConfig: AgentOsActorConfig<TConnParams>,
): void {
	agentOs.onSessionEvent(sessionId, (event) => {
		c.broadcast("sessionEvent", { sessionId, event });

		// Persist event to SQLite for sleep/wake recovery.
		persistSessionEvent(c, sessionId, event).catch((err) =>
			c.log.error({
				msg: "agent-os failed to persist session event",
				sessionId,
				error: err,
			}),
		);

		if (parsedConfig.onSessionEvent) {
			runHook(c, "onSessionEvent", () =>
				parsedConfig.onSessionEvent?.(c, sessionId, event),
			);
		}
	});

	agentOs.onPermissionRequest(sessionId, (request) => {
		c.broadcast("permissionRequest", { sessionId, request });

		if (parsedConfig.onPermissionRequest) {
			runHook(c, "onPermissionRequest", () =>
				parsedConfig.onPermissionRequest?.(c, sessionId, request),
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
			const { sessionId } = await agentOs.createSession(agentType, options);
			subscribeToSession(c, agentOs, sessionId, config);

			// Persist session metadata to SQLite for sleep/wake recovery.
			await persistSession(c, agentOs, sessionId, agentType);

			c.log.info({
				msg: "agent-os session created",
				sessionId,
				agentType,
			});
			return toSessionRecord(agentOs, sessionId, agentType);
		},

		listSessions: async (
			c: AgentOsActionContext<TConnParams>,
		): Promise<SessionInfo[]> => {
			const agentOs = await ensureVm(c, config);
			return agentOs.listSessions();
		},

		getSession: async (
			c: AgentOsActionContext<TConnParams>,
			sessionId: string,
		): Promise<SessionRecord> => {
			assertSessionExists(c, sessionId);
			const agentOs = await ensureVm(c, config);
			const info = agentOs.listSessions().find((s) => s.sessionId === sessionId);
			if (!info) {
				throw new Error(`session not found: ${sessionId}`);
			}
			return toSessionRecord(agentOs, sessionId, info.agentType);
		},

		destroySession: async (
			c: AgentOsActionContext<TConnParams>,
			sessionId: string,
		): Promise<void> => {
			const agentOs = await ensureVm(c, config);
			await agentOs.destroySession(sessionId);
			c.vars.sessions.delete(sessionId);
			c.vars.activeSessionIds.delete(sessionId);
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
			return agentOs.resumeSession(sessionId);
		},

		closeSession: async (
			c: AgentOsActionContext<TConnParams>,
			sessionId: string,
		): Promise<void> => {
			const agentOs = await ensureVm(c, config);
			agentOs.closeSession(sessionId);
			c.vars.sessions.delete(sessionId);
			c.vars.activeSessionIds.delete(sessionId);
			syncPreventSleep(c);

			// Clean up persisted session and events from SQLite.
			await deletePersistedSession(c, sessionId);

			c.log.info({ msg: "agent-os session closed", sessionId });
		},
	};
}

// Build prompt, cancel, and permission actions for the actor factory.
export function buildPromptActions<TConnParams>(
	_config: AgentOsActorConfig<TConnParams>,
) {
	return {
		sendPrompt: async (
			c: AgentOsActionContext<TConnParams>,
			sessionId: string,
			text: string,
		): Promise<JsonRpcResponse> => {
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
			try {
				return await agentOs.prompt(sessionId, text);
			} finally {
				c.vars.activeSessionIds.delete(sessionId);
				syncPreventSleep(c);
				c.log.info({
					msg: "agent-os prompt turn ended",
					sessionId,
					durationMs: Date.now() - start,
				});
			}
		},

		cancelPrompt: async (
			c: AgentOsActionContext<TConnParams>,
			sessionId: string,
		): Promise<JsonRpcResponse> => {
			assertSessionExists(c, sessionId);
			const agentOs = c.vars.agentOs;
			if (!agentOs) {
				throw new Error("VM not initialized");
			}
			return agentOs.cancelSession(sessionId);
		},

		respondPermission: async (
			c: AgentOsActionContext<TConnParams>,
			sessionId: string,
			permissionId: string,
			reply: PermissionReply,
		): Promise<JsonRpcResponse> => {
			assertSessionExists(c, sessionId);
			const agentOs = c.vars.agentOs;
			if (!agentOs) {
				throw new Error("VM not initialized");
			}
			return agentOs.respondPermission(sessionId, permissionId, reply);
		},
	};
}

// Build session configuration proxy actions for the actor factory.
export function buildConfigActions<TConnParams>(
	_config: AgentOsActorConfig<TConnParams>,
) {
	return {
		setMode: async (
			c: AgentOsActionContext<TConnParams>,
			sessionId: string,
			modeId: string,
		): Promise<JsonRpcResponse> => {
			assertSessionExists(c, sessionId);
			const agentOs = c.vars.agentOs;
			if (!agentOs) {
				throw new Error("VM not initialized");
			}
			return agentOs.setSessionMode(sessionId, modeId);
		},

		getModes: async (
			c: AgentOsActionContext<TConnParams>,
			sessionId: string,
		): Promise<SessionModeState | null> => {
			assertSessionExists(c, sessionId);
			const agentOs = c.vars.agentOs;
			if (!agentOs) {
				throw new Error("VM not initialized");
			}
			return agentOs.getSessionModes(sessionId);
		},

		setModel: async (
			c: AgentOsActionContext<TConnParams>,
			sessionId: string,
			model: string,
		): Promise<JsonRpcResponse> => {
			assertSessionExists(c, sessionId);
			const agentOs = c.vars.agentOs;
			if (!agentOs) {
				throw new Error("VM not initialized");
			}
			return agentOs.setSessionModel(sessionId, model);
		},

		setThoughtLevel: async (
			c: AgentOsActionContext<TConnParams>,
			sessionId: string,
			level: string,
		): Promise<JsonRpcResponse> => {
			assertSessionExists(c, sessionId);
			const agentOs = c.vars.agentOs;
			if (!agentOs) {
				throw new Error("VM not initialized");
			}
			return agentOs.setSessionThoughtLevel(sessionId, level);
		},

		getConfigOptions: async (
			c: AgentOsActionContext<TConnParams>,
			sessionId: string,
		): Promise<SessionConfigOption[]> => {
			assertSessionExists(c, sessionId);
			const agentOs = c.vars.agentOs;
			if (!agentOs) {
				throw new Error("VM not initialized");
			}
			return agentOs.getSessionConfigOptions(sessionId);
		},

		getEvents: async (
			c: AgentOsActionContext<TConnParams>,
			sessionId: string,
			options?: GetEventsOptions,
		): Promise<JsonRpcNotification[]> => {
			assertSessionExists(c, sessionId);
			const agentOs = c.vars.agentOs;
			if (!agentOs) {
				throw new Error("VM not initialized");
			}
			return agentOs
				.getSessionEvents(sessionId, options)
				.map((e) => e.notification);
		},

		getSequencedEvents: async (
			c: AgentOsActionContext<TConnParams>,
			sessionId: string,
			options?: GetEventsOptions,
		): Promise<SequencedEvent[]> => {
			assertSessionExists(c, sessionId);
			const agentOs = c.vars.agentOs;
			if (!agentOs) {
				throw new Error("VM not initialized");
			}
			return agentOs.getSessionEvents(sessionId, options);
		},

		rawSend: async (
			c: AgentOsActionContext<TConnParams>,
			sessionId: string,
			method: string,
			params?: Record<string, unknown>,
		): Promise<JsonRpcResponse> => {
			assertSessionExists(c, sessionId);
			const agentOs = c.vars.agentOs;
			if (!agentOs) {
				throw new Error("VM not initialized");
			}
			return agentOs.rawSessionSend(sessionId, method, params);
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
				event: JSON.parse(row.event),
				createdAt: row.created_at,
			}));
		},
	};
}
