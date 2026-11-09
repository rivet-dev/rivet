import { openai } from "@ai-sdk/openai";
import { type CoreMessage, streamText } from "ai";
import { type ActorContextOf, actor, event, setup, UserError } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

export type AgentMessage = {
	id: string;
	role: "user" | "assistant";
	sender: string;
	content: string;
	createdAt: number;
	// A streaming reply names the message it answers, so clients can show it
	// right after that message, where `getHistory` lists it.
	replyTo?: string;
};

export type AgentStatus = {
	state: "idle" | "thinking" | "error";
	updatedAt: number;
	error?: string;
};

export type AgentInfo = {
	id: string;
	name: string;
	createdAt: number;
};

export type AgentMessageInput = {
	text: string;
	sender?: string;
};

export type AgentResponseEvent = {
	messageId: string;
	delta: string;
	content: string;
	done: boolean;
	error?: string;
};

type AgentVars = {
	// The status is a UI signal that describes the current run.
	status: AgentStatus;
	// `getHistory` reads the reply that is streaming right now to include its
	// partial content.
	reply: AgentMessage | null;
	// A new message sets this flag so that a running drain checks for it
	// before it stops.
	pending: boolean;
	// The running drain is kept here so that only one runs at a time.
	draining: Promise<void> | null;
	// `onWake` sets this function before any action runs, and every drain
	// starts through it.
	startDrain: () => void;
};

const SYSTEM_PROMPT =
	"You are a focused AI assistant. Keep responses concise, actionable, and ready for handoff.";

const SELECT_MESSAGES =
	"SELECT id, role, sender, content, created_at AS createdAt FROM messages ORDER BY seq";

const SELECT_PENDING_MESSAGES =
	"SELECT id, 'user' AS role, sender, content, created_at AS createdAt FROM pending_messages ORDER BY seq";

const buildId = (prefix: string) =>
	`${prefix}-${Date.now()}-${Math.random().toString(16).slice(2)}`;

const buildPromptMessages = (messages: AgentMessage[]): CoreMessage[] => {
	return [
		{ role: "system", content: SYSTEM_PROMPT },
		...messages.map((message) => ({
			role: message.role,
			content: message.content,
		})),
	];
};

const saveMessage = async (db: RawAccess, message: AgentMessage) => {
	await db.execute(
		"INSERT INTO messages (id, role, sender, content, created_at) VALUES (?, ?, ?, ?, ?)",
		message.id,
		message.role,
		message.sender,
		message.content,
		message.createdAt,
	);
};

export const agent = actor({
	// Messages are saved in the actor's SQLite database, which survives
	// restarts: https://rivet.dev/docs/actors/sqlite
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS messages (
					seq INTEGER PRIMARY KEY AUTOINCREMENT,
					id TEXT NOT NULL UNIQUE,
					role TEXT NOT NULL,
					sender TEXT NOT NULL,
					content TEXT NOT NULL,
					created_at INTEGER NOT NULL
				)
			`);
			// User messages wait here until their reply is saved, so a message
			// sent before a crash or sleep is still answered afterwards.
			await db.execute(`
				CREATE TABLE IF NOT EXISTS pending_messages (
					seq INTEGER PRIMARY KEY AUTOINCREMENT,
					id TEXT NOT NULL UNIQUE,
					sender TEXT NOT NULL,
					content TEXT NOT NULL,
					created_at INTEGER NOT NULL
				)
			`);
		},
	}),

	// The status, the reply in progress and the drain only describe the
	// current run, so they live in memory: https://rivet.dev/docs/actors/state
	createVars: (): AgentVars => ({
		status: { state: "idle", updatedAt: Date.now() },
		reply: null,
		pending: false,
		draining: null,
		startDrain: () => {
			throw new Error("onWake has not run yet");
		},
	}),
	events: {
		messageAdded: event<AgentMessage>(),
		status: event<AgentStatus>(),
		response: event<AgentResponseEvent>(),
	},

	// `onWake` runs on every start, so messages that were still pending when
	// the actor last slept or crashed get answered: https://rivet.dev/docs/actors/lifecycle
	onWake: (c) => {
		// Drains run on this context because an action's `c.abortSignal` stops
		// firing on shutdown once that action returns.
		c.vars.startDrain = () => startDrain(c);
		c.vars.startDrain();
	},

	actions: {
		// Callable functions from clients: https://rivet.dev/docs/actors/actions
		sendMessage: async (c, input: AgentMessageInput) => {
			// Clients are untrusted, so check the payload before saving it
			const text =
				typeof input?.text === "string" ? input.text.trim() : "";
			if (!text) {
				throw new UserError("Message text is required");
			}

			const sender = input.sender?.trim() || "Operator";
			const userMessage: AgentMessage = {
				id: buildId("user"),
				role: "user",
				sender,
				content: text,
				createdAt: Date.now(),
			};

			// The message is durable once this insert resolves. The drain
			// answers it after every message that arrived before it.
			await c.db.execute(
				"INSERT INTO pending_messages (id, sender, content, created_at) VALUES (?, ?, ?, ?)",
				userMessage.id,
				userMessage.sender,
				userMessage.content,
				userMessage.createdAt,
			);
			c.broadcast("messageAdded", userMessage);

			c.vars.startDrain();
			return userMessage;
		},
		// The explicit return type avoids a circular type, because
		// `readHistory` takes this actor's context
		getHistory: (c): Promise<AgentMessage[]> => readHistory(c),
		getStatus: (c) => c.vars.status,
	},
});

/**
 * Starts a drain unless one is already running. `keepAwake` stops the actor
 * from sleeping while a reply is streaming. Call it through
 * `c.vars.startDrain`, which passes the `onWake` context.
 */
function startDrain(c: ActorContextOf<typeof agent>): void {
	c.vars.pending = true;
	if (c.vars.draining) {
		return;
	}

	c.vars.draining = c
		.keepAwake(answerPendingMessages(c))
		.catch((error) => {
			// The message stays pending and is answered on the next drain
			c.log.error({ msg: "failed to answer message", error });
		})
		.finally(() => {
			c.vars.draining = null;
			// Catch messages that arrived after the drain last checked `pending`
			if (c.vars.pending && !c.aborted) {
				startDrain(c);
			}
		});
}

/**
 * Answers pending messages oldest first, one model call at a time, until
 * none are left.
 */
async function answerPendingMessages(
	c: ActorContextOf<typeof agent>,
): Promise<void> {
	while (c.vars.pending) {
		c.vars.pending = false;

		// Stop at shutdown. The message being answered stays pending for the
		// next start.
		while (!c.aborted) {
			const [userMessage] = await c.db.execute<AgentMessage>(
				`${SELECT_PENDING_MESSAGES} LIMIT 1`,
			);
			if (!userMessage) {
				break;
			}

			await answerMessage(c, userMessage);
		}
	}
}

/**
 * Streams a reply to one pending message. The reply is saved together with
 * the message it answers, so a crash or shutdown before then leaves the
 * message pending and the next start answers it again from the beginning.
 */
async function answerMessage(
	c: ActorContextOf<typeof agent>,
	userMessage: AgentMessage,
): Promise<void> {
	const history = await c.db.execute<AgentMessage>(SELECT_MESSAGES);
	const promptMessages = buildPromptMessages([...history, userMessage]);

	const assistantMessage: AgentMessage = {
		id: buildId("assistant"),
		role: "assistant",
		sender: "Agent",
		content: "",
		createdAt: Date.now(),
		replyTo: userMessage.id,
	};

	c.vars.reply = assistantMessage;
	c.broadcast("messageAdded", assistantMessage);

	c.vars.status = {
		state: "thinking",
		updatedAt: Date.now(),
	};
	c.broadcast("status", c.vars.status);

	try {
		const result = await streamText({
			model: openai("gpt-4o-mini"),
			messages: promptMessages,
			// Shutdown cancels the model call
			abortSignal: c.abortSignal,
		});

		let content = "";
		for await (const delta of result.textStream) {
			content += delta;
			assistantMessage.content = content;
			c.broadcast("response", {
				messageId: assistantMessage.id,
				delta,
				content,
				done: false,
			});
		}

		// A reply cut short by shutdown is not saved
		if (c.aborted) {
			return;
		}

		await saveReply(c, userMessage, assistantMessage);
		c.broadcast("response", {
			messageId: assistantMessage.id,
			delta: "",
			content: assistantMessage.content,
			done: true,
		});

		c.vars.status = {
			state: "idle",
			updatedAt: Date.now(),
		};
		c.broadcast("status", c.vars.status);
	} catch (error) {
		if (c.aborted) {
			return;
		}

		const errorMessage =
			error instanceof Error ? error.message : "Unknown error";

		assistantMessage.content =
			assistantMessage.content ||
			"I hit a snag while responding. Please try again.";
		await saveReply(c, userMessage, assistantMessage);

		c.vars.status = {
			state: "error",
			updatedAt: Date.now(),
			error: errorMessage,
		};

		c.broadcast("response", {
			messageId: assistantMessage.id,
			delta: "",
			content: assistantMessage.content,
			done: true,
			error: errorMessage,
		});
		c.broadcast("status", c.vars.status);
	} finally {
		c.vars.reply = null;
	}
}

/** Moves a pending message and its reply into the transcript atomically. */
async function saveReply(
	c: ActorContextOf<typeof agent>,
	userMessage: AgentMessage,
	assistantMessage: AgentMessage,
): Promise<void> {
	await c.db.transaction(
		async (tx) => {
			await saveMessage(tx, userMessage);
			await saveMessage(tx, assistantMessage);
			await tx.execute(
				"DELETE FROM pending_messages WHERE id = ?",
				userMessage.id,
			);
		},
		{ name: "save-reply" },
	);
}

/**
 * Returns the saved transcript followed by the pending messages, with the
 * reply that is streaming right now after the message it answers.
 */
async function readHistory(
	c: ActorContextOf<typeof agent>,
): Promise<AgentMessage[]> {
	// No reply can be saved while this transaction runs, so the reply in
	// `c.vars` is listed only while its message is still pending.
	return c.db.transaction(
		async (tx) => {
			const saved = await tx.execute<AgentMessage>(SELECT_MESSAGES);
			const pending = await tx.execute<AgentMessage>(
				SELECT_PENDING_MESSAGES,
			);

			const history = [...saved];
			for (const message of pending) {
				history.push(message);

				const reply = c.vars.reply;
				if (reply?.replyTo === message.id) {
					history.push(reply);
				}
			}
			return history;
		},
		{ name: "read-history" },
	);
}

export const agentManager = actor({
	// The agent list is saved in SQLite, which survives restarts: https://rivet.dev/docs/actors/sqlite
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS agents (
					seq INTEGER PRIMARY KEY AUTOINCREMENT,
					id TEXT NOT NULL UNIQUE,
					name TEXT NOT NULL,
					created_at INTEGER NOT NULL
				)
			`);
		},
	}),

	actions: {
		// Callable functions from clients: https://rivet.dev/docs/actors/actions
		createAgent: async (c, name?: string) => {
			const trimmedName = name?.trim();

			// Count and insert in one transaction so concurrent calls never pick
			// the same default name.
			const info = await c.db.transaction(
				async (tx) => {
					const [{ agentCount }] = await tx.execute<{
						agentCount: number;
					}>("SELECT COUNT(*) AS agentCount FROM agents");
					const agentInfo: AgentInfo = {
						id: buildId("agent"),
						name: trimmedName || `Agent ${agentCount + 1}`,
						createdAt: Date.now(),
					};
					await tx.execute(
						"INSERT INTO agents (id, name, created_at) VALUES (?, ?, ?)",
						agentInfo.id,
						agentInfo.name,
						agentInfo.createdAt,
					);
					return agentInfo;
				},
				{ name: "create-agent" },
			);

			const client = c.client<typeof registry>();
			const handle = client.agent.getOrCreate([info.id]);
			await handle.getStatus();

			return info;
		},

		listAgents: (c) =>
			c.db.execute<AgentInfo>(
				"SELECT id, name, created_at AS createdAt FROM agents ORDER BY seq",
			),
	},
});

// Register actors for use: https://rivet.dev/docs/setup
export const registry = setup({
	use: { agent, agentManager },
});

// Start the server on port 6420
registry.start();
