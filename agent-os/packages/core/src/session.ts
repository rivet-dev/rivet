// Session lifecycle management for ACP agent sessions

import type { AcpClient, NotificationHandler } from "./acp-client.js";
import type { JsonRpcNotification, JsonRpcResponse } from "./protocol.js";

export type SessionEventHandler = (event: JsonRpcNotification) => void;

/** Permission request from an agent (e.g., before running a shell command or editing a file). */
export interface PermissionRequest {
	/** Unique identifier for this permission request. */
	permissionId: string;
	/** Description of what the agent wants to do. */
	description?: string;
	/** The raw params from the JSON-RPC notification. */
	params: Record<string, unknown>;
}

/** Reply to a permission request. */
export type PermissionReply = "once" | "always" | "reject";

export type PermissionRequestHandler = (request: PermissionRequest) => void;

/** A mode the agent supports (e.g., "plan", "normal", "full-access"). */
export interface SessionMode {
	id: string;
	label?: string;
	description?: string;
}

/** Current mode state reported by the agent. */
export interface SessionModeState {
	currentModeId: string;
	availableModes: SessionMode[];
}

/** A configuration option the agent supports. */
export interface SessionConfigOption {
	id: string;
	category?: string;
	label?: string;
	description?: string;
	currentValue?: string;
	allowedValues?: Array<{ id: string; label?: string }>;
}

/** Boolean capability flags reported by the agent during initialize. */
export interface AgentCapabilities {
	permissions?: boolean;
	plan_mode?: boolean;
	questions?: boolean;
	tool_calls?: boolean;
	text_messages?: boolean;
	images?: boolean;
	file_attachments?: boolean;
	session_lifecycle?: boolean;
	error_events?: boolean;
	reasoning?: boolean;
	status?: boolean;
	streaming_deltas?: boolean;
	mcp_tools?: boolean;
}

/** Agent identity information from the initialize response. */
export interface AgentInfo {
	name: string;
	version?: string;
}

/** Options for constructing a Session, including capabilities from initialize/session-new. */
export interface SessionInitData {
	modes?: SessionModeState;
	configOptions?: SessionConfigOption[];
	capabilities?: AgentCapabilities;
	agentInfo?: AgentInfo;
}

/** A notification with an assigned sequence number for ordering. */
export interface SequencedEvent {
	sequenceNumber: number;
	notification: JsonRpcNotification;
}

/** Options for filtering event history. */
export interface GetEventsOptions {
	/** Return only events with sequence number greater than this value. */
	since?: number;
	/** Return only events with this JSON-RPC method. */
	method?: string;
}

export class Session {
	private _client: AcpClient;
	private _sessionId: string;
	private _agentType: string;
	private _eventHandlers: SessionEventHandler[] = [];
	private _permissionHandlers: PermissionRequestHandler[] = [];
	private _modes: SessionModeState | null;
	private _configOptions: SessionConfigOption[];
	private _capabilities: AgentCapabilities;
	private _agentInfo: AgentInfo | null;
	private _events: SequencedEvent[] = [];
	private _nextSequence = 0;
	private _closed = false;
	private _onClose?: () => void;

	constructor(
		client: AcpClient,
		sessionId: string,
		agentType: string,
		initData?: SessionInitData,
		onClose?: () => void,
	) {
		this._client = client;
		this._sessionId = sessionId;
		this._agentType = agentType;
		this._onClose = onClose;
		this._modes = initData?.modes ?? null;
		this._configOptions = initData?.configOptions ?? [];
		this._capabilities = initData?.capabilities ?? {};
		this._agentInfo = initData?.agentInfo ?? null;

		// Forward notifications to appropriate handlers and store in event history
		const handler: NotificationHandler = (notification) => {
			// Store all notifications in event history
			this._events.push({
				sequenceNumber: this._nextSequence++,
				notification,
			});

			if (notification.method === "session/update") {
				for (const h of this._eventHandlers) {
					h(notification);
				}
			} else if (notification.method === "request/permission") {
				const params = (notification.params ?? {}) as Record<
					string,
					unknown
				>;
				const request: PermissionRequest = {
					permissionId: params.permissionId as string,
					description: params.description as string | undefined,
					params,
				};
				for (const h of this._permissionHandlers) {
					h(request);
				}
			}
		};
		this._client.onNotification(handler);
	}

	get sessionId(): string {
		return this._sessionId;
	}

	get agentType(): string {
		return this._agentType;
	}

	/** Agent capability flags from the initialize response. */
	get capabilities(): AgentCapabilities {
		return this._capabilities;
	}

	/** Agent identity information from the initialize response. */
	get agentInfo(): AgentInfo | null {
		return this._agentInfo;
	}

	/** Whether this session has been closed. */
	get closed(): boolean {
		return this._closed;
	}

	private _throwIfClosed(): void {
		if (this._closed) {
			throw new Error(`Session ${this._sessionId} is closed`);
		}
	}

	/**
	 * Send a prompt to the agent and wait for the final response.
	 * Session/update notifications arrive via onSessionEvent() while this resolves.
	 */
	async prompt(text: string): Promise<JsonRpcResponse> {
		this._throwIfClosed();
		return this._client.request("session/prompt", {
			sessionId: this._sessionId,
			prompt: [{ type: "text", text }],
		});
	}

	/** Subscribe to session/update notifications from the agent. */
	onSessionEvent(handler: SessionEventHandler): void {
		this._eventHandlers.push(handler);
	}

	/** Remove a previously registered session event handler. */
	removeSessionEventHandler(handler: SessionEventHandler): void {
		const idx = this._eventHandlers.indexOf(handler);
		if (idx !== -1) {
			this._eventHandlers.splice(idx, 1);
		}
	}

	/** Subscribe to permission requests from the agent. */
	onPermissionRequest(handler: PermissionRequestHandler): void {
		this._permissionHandlers.push(handler);
	}

	/** Remove a previously registered permission request handler. */
	removePermissionRequestHandler(handler: PermissionRequestHandler): void {
		const idx = this._permissionHandlers.indexOf(handler);
		if (idx !== -1) {
			this._permissionHandlers.splice(idx, 1);
		}
	}

	/**
	 * Respond to a permission request from the agent.
	 * @param permissionId - The ID from the PermissionRequest
	 * @param reply - 'once' to allow this action, 'always' to always allow, 'reject' to deny
	 */
	async respondPermission(
		permissionId: string,
		reply: PermissionReply,
	): Promise<JsonRpcResponse> {
		this._throwIfClosed();
		return this._client.request("request/permission", {
			sessionId: this._sessionId,
			permissionId,
			reply,
		});
	}

	/**
	 * Set the session mode (e.g., "plan", "normal").
	 * Sends session/set_mode via ACP.
	 */
	async setMode(modeId: string): Promise<JsonRpcResponse> {
		this._throwIfClosed();
		return this._client.request("session/set_mode", {
			sessionId: this._sessionId,
			modeId,
		});
	}

	/** Returns available modes from the agent's reported capabilities. */
	getModes(): SessionModeState | null {
		return this._modes;
	}

	/**
	 * Set the model for this session.
	 * Finds the config option with category "model" and sends session/set_config_option.
	 */
	async setModel(model: string): Promise<JsonRpcResponse> {
		return this._setConfigByCategory("model", model);
	}

	/**
	 * Set the thought/reasoning level for this session.
	 * Finds the config option with category "thought_level" and sends session/set_config_option.
	 */
	async setThoughtLevel(level: string): Promise<JsonRpcResponse> {
		return this._setConfigByCategory("thought_level", level);
	}

	/** Returns available config options from the agent. */
	getConfigOptions(): SessionConfigOption[] {
		return this._configOptions;
	}

	/**
	 * Send session/set_config_option for a config option identified by category.
	 * If no matching config option is found, sends with the category as the configId.
	 */
	private async _setConfigByCategory(
		category: string,
		value: string,
	): Promise<JsonRpcResponse> {
		this._throwIfClosed();
		const option = this._configOptions.find((o) => o.category === category);
		const configId = option?.id ?? category;
		return this._client.request("session/set_config_option", {
			sessionId: this._sessionId,
			configId,
			value,
		});
	}

	/**
	 * Returns the event history as an array of JsonRpcNotification objects.
	 * Supports optional filtering by sequence number and method.
	 */
	getEvents(options?: GetEventsOptions): JsonRpcNotification[] {
		let events = this._events;
		const since = options?.since;
		const method = options?.method;

		if (since !== undefined) {
			events = events.filter((e) => e.sequenceNumber > since);
		}
		if (method !== undefined) {
			events = events.filter((e) => e.notification.method === method);
		}

		return events.map((e) => e.notification);
	}

	/**
	 * Returns the full sequenced event history.
	 * Each entry includes the notification and its sequence number.
	 */
	getSequencedEvents(options?: GetEventsOptions): SequencedEvent[] {
		let events = this._events;
		const since = options?.since;
		const method = options?.method;

		if (since !== undefined) {
			events = events.filter((e) => e.sequenceNumber > since);
		}
		if (method !== undefined) {
			events = events.filter((e) => e.notification.method === method);
		}

		return [...events];
	}

	/** Cancel ongoing agent work for this session. */
	async cancel(): Promise<JsonRpcResponse> {
		this._throwIfClosed();
		return this._client.request("session/cancel", {
			sessionId: this._sessionId,
		});
	}

	/**
	 * Send an arbitrary JSON-RPC request to the agent.
	 * Automatically injects sessionId into params if not already present.
	 * Use this for ACP methods that don't have typed wrappers yet.
	 */
	async rawSend(
		method: string,
		params?: Record<string, unknown>,
	): Promise<JsonRpcResponse> {
		this._throwIfClosed();
		const mergedParams = { sessionId: this._sessionId, ...params };
		return this._client.request(method, mergedParams);
	}

	/** Kill the agent process and clear event history. */
	close(): void {
		if (this._closed) return;
		this._closed = true;
		this._events = [];
		this._client.close();
		this._onClose?.();
	}
}
