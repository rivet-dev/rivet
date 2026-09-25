import { randomUUID } from "node:crypto";
import {
	type Agent,
	type AgentSideConnection,
	type AuthMethod,
	type InitializeResponse,
	type LoadSessionRequest,
	type NewSessionRequest,
	PROTOCOL_VERSION,
	type PromptRequest,
	type PromptResponse,
	RequestError,
	type SessionConfigOption,
	type SessionUpdate,
	type SetSessionConfigOptionRequest,
	type StopReason,
} from "@agentclientprotocol/sdk";
import type { AgentSession, AgentSessionEvent } from "@earendil-works/pi-coding-agent";
import type { PiSessionInfo } from "../actions.js";
import type { PiModelInfo } from "../models.js";
import { PiEventTranslator, replayMessages, toPiPrompt } from "./updates.js";

/** The Pi actor actions the bridge calls, over one connection per ACP session. */
export interface PiAgentConnection {
	prompt(
		text: string,
		options: { images: { type: "image"; data: string; mimeType: string }[] },
	): Promise<void>;
	abort(): Promise<void>;
	getSession(): Promise<PiSessionInfo>;
	getMessages(): Promise<AgentSession["messages"]>;
	getAvailableModels(): Promise<PiModelInfo[]>;
	setModel(provider: string, modelId: string): Promise<void>;
	getAvailableThinkingLevels(): Promise<string[]>;
	setThinkingLevel(level: string): Promise<void>;
	waitForIdle(): Promise<void>;
	on(name: "event", callback: (event: AgentSessionEvent) => void): unknown;
	onStatusChange(callback: (status: ConnectionStatus) => void): unknown;
	dispose(): Promise<void>;
}

/** RivetKit's connection status. `idle` after a connection means RivetKit stopped reconnecting. */
type ConnectionStatus = "idle" | "connecting" | "connected" | "disconnected";

/** A Pi actor handle: one stateless call, or a connection for streaming events. */
export interface PiAgentHandle {
	getSession(): Promise<PiSessionInfo>;
	connect(): PiAgentConnection;
}

export interface PiAcpOptions {
	/**
	 * The Pi actor for an ACP session id. `create` is false for `session/load`.
	 * An application can resolve it asynchronously, for example by asking its
	 * backend for the actor id and a token scoped to that actor. An error thrown
	 * here is shown to the user.
	 */
	actor(sessionId: string, create: boolean): PiAgentHandle | Promise<PiAgentHandle>;
	/**
	 * The application's terminal login, which an editor offers when a new
	 * session has no model with a credential: the command that starts this
	 * bridge and the arguments that run the login instead.
	 */
	login?: { command: string; commandArgs: string[]; loginArgs: string[] };
	/**
	 * How long `session/new` and `session/load` wait for the Pi actor to answer.
	 * RivetKit's client retries an unreachable engine forever. Defaults to 30 seconds.
	 */
	openTimeoutMs?: number;
}

const DEFAULT_OPEN_TIMEOUT_MS = 30_000;

class OpenTimeoutError extends Error {}

interface Turn {
	/** Pi started this turn's run: its `agent_start` arrived. */
	started: boolean;
	cancelled: boolean;
	/** The turn's last assistant message. A retry replaces a failed attempt. */
	last: { stopReason: string; errorMessage?: string } | undefined;
	/** The connection dropped during the turn. Events sent in the gap are not replayed. */
	missedEvents: boolean;
}

class AcpSession {
	readonly translator = new PiEventTranslator();
	turn: Turn | undefined;
	/** Serializes notifications so the client sees them in Pi's order. */
	sent: Promise<void> = Promise.resolve();

	constructor(
		readonly id: string,
		readonly conn: PiAgentConnection,
	) {}
}

const LOGIN_METHOD_ID = "rivet-pi-login";

/**
 * An ACP agent backed by Pi actors. Each ACP session is one Pi actor. Tools
 * run where the actor runs them, in its sandbox; the editor shows the chat,
 * tool calls, and diffs.
 */
export class PiAcpAgent implements Agent {
	readonly #client: AgentSideConnection;
	readonly #options: PiAcpOptions;
	readonly #sessions = new Map<string, AcpSession>();

	constructor(client: AgentSideConnection, options: PiAcpOptions) {
		this.#client = client;
		this.#options = options;
	}

	async initialize(): Promise<InitializeResponse> {
		return {
			protocolVersion: PROTOCOL_VERSION,
			agentInfo: { name: "rivet-pi", title: "Pi on Rivet", version: "0.0.0" },
			authMethods: this.#authMethods(),
			agentCapabilities: {
				loadSession: true,
				promptCapabilities: { image: true, embeddedContext: true },
			},
		};
	}

	async authenticate(): Promise<void> {
	}

	async newSession(_params: NewSessionRequest) {
		const session = await this.#open(randomUUID(), true);
		const configOptions = await this.#configOptions(session);
		const login = this.#options.login;
		if (login && !configOptions.some((option) => option.id === "model")) {
			await this.#close(session.id);
			const command = [login.command, ...login.commandArgs, ...login.loginArgs].map(shellQuote).join(" ");
			throw RequestError.authRequired(
				{ authMethods: this.#authMethods() },
				`Log in to a model provider first. Run \`${command}\` in a terminal.`,
			);
		}
		return { sessionId: session.id, configOptions };
	}

	async loadSession(params: LoadSessionRequest) {
		await this.#close(params.sessionId);
		const session = await this.#open(params.sessionId, false);
		for (const update of replayMessages(await session.conn.getMessages())) {
			this.#send(session, update);
		}
		await session.sent;
		return { configOptions: await this.#configOptions(session) };
	}

	async prompt(params: PromptRequest): Promise<PromptResponse> {
		const session = this.#sessions.get(params.sessionId) ?? (await this.#session(params.sessionId));
		if (session.turn) {
			throw RequestError.invalidRequest({}, "A prompt is already running in this session.");
		}
		const { text, images } = toPiPrompt(params.prompt);
		const turn: Turn = { started: false, cancelled: false, last: undefined, missedEvents: false };
		session.turn = turn;
		let result: StopReason | Error;
		try {
			result = await this.#run(session, turn, text, images);
		} finally {
			session.turn = undefined;
		}
		await session.sent;
		if (result instanceof Error) throw result;
		return { stopReason: result };
	}

	async cancel(params: { sessionId: string }): Promise<void> {
		const session = this.#sessions.get(params.sessionId);
		const turn = session?.turn;
		if (!session || !turn) return;
		turn.cancelled = true;
		await session.conn.abort();
	}

	async setSessionConfigOption(params: SetSessionConfigOptionRequest) {
		const session = await this.#session(params.sessionId);
		const value = String(params.value);
		if (params.configId === "model") {
			const slash = value.indexOf("/");
			await session.conn.setModel(value.slice(0, slash), value.slice(slash + 1));
		} else if (params.configId === "thinking") {
			await session.conn.setThinkingLevel(value);
		} else {
			throw RequestError.invalidParams({}, `Unknown config option ${params.configId}.`);
		}
		return { configOptions: await this.#configOptions(session) };
	}

	/** Closes every actor connection. */
	async dispose(): Promise<void> {
		await Promise.all([...this.#sessions.keys()].map((id) => this.#close(id)));
	}

	async #open(sessionId: string, create: boolean): Promise<AcpSession> {
		let handle: PiAgentHandle;
		try {
			handle = await this.#options.actor(sessionId, create);
		} catch (error) {
			throw RequestError.internalError({}, errorMessage(error));
		}
		const timeoutMs = this.#options.openTimeoutMs ?? DEFAULT_OPEN_TIMEOUT_MS;
		try {
			await withTimeout(handle.getSession(), timeoutMs);
		} catch (error) {
			if (error instanceof OpenTimeoutError) {
				throw RequestError.internalError(
					{},
					`The Pi actor did not answer within ${timeoutMs / 1000} s. Check that the Rivet engine at RIVET_ENDPOINT is running.`,
				);
			}
			throw !create && isActorNotFound(error)
				? RequestError.resourceNotFound(sessionId)
				: RequestError.internalError({}, errorMessage(error));
		}
		const session = new AcpSession(sessionId, handle.connect());
		session.conn.on("event", (event) => this.#onEvent(session, event));
		session.conn.onStatusChange((status) => this.#onStatus(session, status));
		this.#sessions.set(sessionId, session);
		return session;
	}

	async #close(sessionId: string): Promise<void> {
		const session = this.#sessions.get(sessionId);
		if (!session) return;
		this.#sessions.delete(sessionId);
		await session.conn.dispose();
	}

	/** The open session, or a new connection to its actor after the previous one was lost. */
	async #session(sessionId: string): Promise<AcpSession> {
		return this.#sessions.get(sessionId) ?? (await this.#open(sessionId, false));
	}

	/**
	 * Runs the prompt action. Its reply ends the turn. The actor sends a run's
	 * events before the reply on this connection, so they have all arrived unless
	 * the connection dropped during the turn.
	 */
	async #run(
		session: AcpSession,
		turn: Turn,
		text: string,
		images: { type: "image"; data: string; mimeType: string }[],
	): Promise<StopReason | Error> {
		try {
			await session.conn.prompt(text, { images });
		} catch (error) {
			return turn.started ? interrupted() : RequestError.internalError({}, errorMessage(error));
		}
		return turn.missedEvents ? interrupted() : outcomeOf(turn, turn.last);
	}

	#onStatus(session: AcpSession, status: ConnectionStatus): void {
		if (this.#sessions.get(session.id) !== session) return;
		switch (status) {
			case "disconnected":
				if (session.turn) session.turn.missedEvents = true;
				break;
			case "idle":
				this.#sessions.delete(session.id);
				void session.conn.dispose().catch(() => {});
				break;
			case "connecting":
			case "connected":
				break;
		}
	}

	#onEvent(session: AcpSession, event: AgentSessionEvent): void {
		for (const update of session.translator.translate(event)) this.#send(session, update);
		const turn = session.turn;
		if (!turn) return;
		if (event.type === "agent_start") {
			turn.started = true;
			if (turn.cancelled) void session.conn.abort().catch(() => {});
		} else if (event.type === "message_end" && event.message.role === "assistant") {
			turn.last = event.message;
		}
	}

	#send(session: AcpSession, update: SessionUpdate): void {
		session.sent = session.sent
			.then(() => this.#client.sessionUpdate({ sessionId: session.id, update }))
			.catch(() => {
			});
	}

	async #configOptions(session: AcpSession): Promise<SessionConfigOption[]> {
		const [info, models, levels] = await Promise.all([
			session.conn.getSession(),
			session.conn.getAvailableModels(),
			session.conn.getAvailableThinkingLevels(),
		]);
		const options: SessionConfigOption[] = [];
		if (models.length > 0) {
			options.push({
				id: "model",
				name: "Model",
				category: "model",
				type: "select",
				currentValue: info.model ? `${info.model.provider}/${info.model.id}` : "",
				options: models.map((model) => ({ value: `${model.provider}/${model.id}`, name: model.name })),
			});
		}
		if (levels.length > 1) {
			options.push({
				id: "thinking",
				name: "Thinking",
				category: "thought_level",
				type: "select",
				currentValue: info.thinkingLevel,
				options: levels.map((level) => ({ value: level, name: level })),
			});
		}
		return options;
	}

	#authMethods(): AuthMethod[] {
		const login = this.#options.login;
		if (!login) return [];
		return [
			{
				id: LOGIN_METHOD_ID,
				name: "Log in to a model provider",
				description: "Log in with a subscription or an API key.",
				type: "terminal",
				args: login.loginArgs,
				_meta: {
					"terminal-auth": {
						command: login.command,
						args: [...login.commandArgs, ...login.loginArgs],
						label: "Log in",
					},
				},
			},
		];
	}
}

async function withTimeout<T>(promise: Promise<T>, ms: number): Promise<T> {
	let timer: NodeJS.Timeout | undefined;
	const timeout = new Promise<never>((_, reject) => {
		timer = setTimeout(() => reject(new OpenTimeoutError()), ms);
	});
	try {
		return await Promise.race([promise, timeout]);
	} finally {
		clearTimeout(timer);
	}
}

/** The connection dropped during a run, so its result is unknown. The prompt is never sent again. */
function interrupted(): Error {
	return RequestError.internalError(
		{},
		"Lost the connection to the Pi actor during this prompt, so its result is unknown. Reload the session to see what ran.",
	);
}

/** A turn's ACP result from its last assistant message. */
function outcomeOf(
	turn: Turn,
	last: { stopReason: string; errorMessage?: string } | undefined,
): StopReason | Error {
	if (turn.cancelled || last?.stopReason === "aborted") return "cancelled";
	if (last?.stopReason === "error") {
		return RequestError.internalError({}, last.errorMessage ?? "The model request failed.");
	}
	return "end_turn";
}

function errorMessage(error: unknown): string {
	return error instanceof Error ? error.message : String(error);
}

function isActorNotFound(error: unknown): boolean {
	const { group, code } = (error ?? {}) as { group?: unknown; code?: unknown };
	return group === "actor" && code === "not_found";
}

/** Quotes one argument for a POSIX shell, for example a path with a space. */
function shellQuote(arg: string): string {
	return /^[\w@%+=:,./-]+$/.test(arg) ? arg : `'${arg.replaceAll("'", `'\\''`)}'`;
}
