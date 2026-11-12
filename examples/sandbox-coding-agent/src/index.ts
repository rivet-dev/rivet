import fs from "node:fs";
import path from "node:path";
import { actor, event, setup } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";
import {
	daytona,
	docker,
	e2b,
	type SessionEvent,
	sandboxActor,
} from "rivetkit/sandbox";

export type AgentMessage = {
	id: string;
	role: "user" | "assistant";
	sender: string;
	content: string;
	createdAt: number;
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

export type AgentResponseEvent = {
	messageId: string;
	delta: string;
	content: string;
	done: boolean;
	error?: string;
};

type SessionUpdatePayload = {
	method?: string;
	params?: {
		update?: {
			sessionUpdate?: string;
			content?: {
				type?: string;
				text?: string;
			};
		};
	};
};

const buildId = (prefix: string) =>
	`${prefix}-${Date.now()}-${Math.random().toString(16).slice(2)}`;

function buildSharedEnvRecord(): Record<string, string> {
	const env: Record<string, string> = {};
	if (process.env.ANTHROPIC_API_KEY) {
		env.ANTHROPIC_API_KEY = process.env.ANTHROPIC_API_KEY;
	}
	if (process.env.OPENAI_API_KEY) {
		env.OPENAI_API_KEY = process.env.OPENAI_API_KEY;
	}
	if (process.env.CODEX_API_KEY) {
		env.CODEX_API_KEY = process.env.CODEX_API_KEY;
	}
	return env;
}

function buildSharedEnvList(): string[] {
	return Object.entries(buildSharedEnvRecord()).map(
		([key, value]) => `${key}=${value}`,
	);
}

function buildDockerBinds(): string[] {
	if (!process.env.HOME) {
		return [];
	}

	const codexAuthPath = path.join(process.env.HOME, ".codex", "auth.json");
	if (!fs.existsSync(codexAuthPath)) {
		return [];
	}

	return [`${codexAuthPath}:/root/.codex/auth.json:ro`];
}

function resolveProviderName(): "docker" | "daytona" | "e2b" {
	const provider = process.env.SANDBOX_PROVIDER?.trim().toLowerCase();
	if (provider === "daytona" || provider === "e2b") {
		return provider;
	}
	return "docker";
}

function resolveSessionCwd(): string {
	switch (resolveProviderName()) {
		case "daytona":
			return "/home/daytona";
		case "e2b":
			return "/home/user";
		default:
			return "/root";
	}
}

function createSandboxProvider() {
	switch (resolveProviderName()) {
		case "daytona":
			return daytona({
				create: {
					envVars: buildSharedEnvRecord(),
				},
			});
		case "e2b":
			return e2b({
				create: {
					allowInternetAccess: true,
					envs: buildSharedEnvRecord(),
				},
			});
		default:
			return docker({
				env: buildSharedEnvList(),
				binds: buildDockerBinds(),
			});
	}
}

async function fetchAllSessionEvents(
	sandbox: {
		getEvents(input: {
			sessionId: string;
			cursor?: string;
			limit?: number;
		}): Promise<{ items: SessionEvent[]; nextCursor?: string }>;
	},
	sessionId: string,
): Promise<SessionEvent[]> {
	const events: SessionEvent[] = [];
	let cursor: string | undefined;

	while (true) {
		const page = await sandbox.getEvents({
			sessionId,
			cursor,
			limit: 200,
		});
		events.push(...page.items);
		if (!page.nextCursor) {
			return events;
		}
		cursor = page.nextCursor;
	}
}

function extractAssistantText(
	events: SessionEvent[],
	minEventIndex: number,
): string {
	return events
		.filter((event) => event.eventIndex > minEventIndex)
		.map((event) => event.payload as SessionUpdatePayload)
		.filter((payload) => payload.method === "session/update")
		.map((payload) => {
			const update = payload.params?.update;
			if (
				!update ||
				update.sessionUpdate !== "agent_message_chunk" ||
				update.content?.type !== "text"
			) {
				return "";
			}
			return update.content.text ?? "";
		})
		.join("");
}

const codingSandbox = sandboxActor({
	provider: createSandboxProvider(),
});

const saveMessage = async (db: RawAccess, message: AgentMessage) => {
	await db.execute(
		`INSERT INTO messages (id, role, sender, content, created_at) VALUES (?, ?, ?, ?, ?)
		ON CONFLICT (id) DO UPDATE SET content = excluded.content`,
		message.id,
		message.role,
		message.sender,
		message.content,
		message.createdAt,
	);
};

export const agent = actor({
	// Messages and the sandbox session ID are saved in the actor's SQLite
	// database, which survives restarts: https://rivet.dev/docs/actors/sqlite
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
			await db.execute(`
				CREATE TABLE IF NOT EXISTS session (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					session_id TEXT NOT NULL
				)
			`);
		},
	}),

	// The status only describes the current prompt, so it lives in memory and
	// starts idle.
	createVars: (): { status: AgentStatus } => ({
		status: { state: "idle", updatedAt: Date.now() },
	}),
	events: {
		messageAdded: event<AgentMessage>(),
		status: event<AgentStatus>(),
		response: event<AgentResponseEvent>(),
	},
	actions: {
		getHistory: (c) =>
			c.db.execute<AgentMessage>(
				"SELECT id, role, sender, content, created_at AS createdAt FROM messages ORDER BY seq",
			),
		getStatus: (c) => c.vars.status,
		sendMessage: async (c, input: { text: string; sender?: string }) => {
			const text = input.text.trim();
			if (!text) {
				return;
			}

			const sender = input.sender?.trim() || "Operator";
			const userMessage: AgentMessage = {
				id: buildId("user"),
				role: "user",
				sender,
				content: text,
				createdAt: Date.now(),
			};
			await saveMessage(c.db, userMessage);
			c.broadcast("messageAdded", userMessage);

			const assistantMessage: AgentMessage = {
				id: buildId("assistant"),
				role: "assistant",
				sender: "Sandbox Actor",
				content: "",
				createdAt: Date.now(),
			};
			await saveMessage(c.db, assistantMessage);
			c.broadcast("messageAdded", assistantMessage);

			c.vars.status = {
				state: "thinking",
				updatedAt: Date.now(),
			};
			c.broadcast("status", c.vars.status);

			try {
				const sandbox = c
					.client<typeof registry>()
					.codingSandbox.getOrCreate([c.key[0]]);
				const [session] = await c.db.execute<{ session_id: string }>(
					"SELECT session_id FROM session WHERE id = 1",
				);
				const sessionId = session?.session_id ?? buildId("session");

				if (!session) {
					await sandbox.resumeOrCreateSession({
						id: sessionId,
						agent: process.env.SANDBOX_AGENT_AGENT ?? "codex",
						mode: process.env.SANDBOX_AGENT_MODE,
						model: process.env.SANDBOX_AGENT_MODEL,
						thoughtLevel: process.env.SANDBOX_AGENT_THOUGHT_LEVEL,
						sessionInit: {
							cwd: resolveSessionCwd(),
						},
					});
					await c.db.execute(
						"INSERT OR REPLACE INTO session (id, session_id) VALUES (1, ?)",
						sessionId,
					);
				}

				const previousEvents = await fetchAllSessionEvents(
					sandbox,
					sessionId,
				);
				const previousMaxEventIndex =
					previousEvents.at(-1)?.eventIndex ?? -1;

				const promptResult = (await sandbox.rawSendSessionMethod(
					sessionId,
					"session/prompt",
					{
						sessionId,
						prompt: [{ type: "text", text }],
					},
				)) as {
					response?: {
						stopReason?: string;
					};
				};

				const nextEvents = await fetchAllSessionEvents(
					sandbox,
					sessionId,
				);
				const assistantText = extractAssistantText(
					nextEvents,
					previousMaxEventIndex,
				);
				assistantMessage.content =
					assistantText ||
					`Prompt completed with stop reason: ${promptResult.response?.stopReason ?? "unknown"}`;
				await saveMessage(c.db, assistantMessage);

				c.broadcast("response", {
					messageId: assistantMessage.id,
					delta: assistantMessage.content,
					content: assistantMessage.content,
					done: true,
				});

				c.vars.status = {
					state: "idle",
					updatedAt: Date.now(),
				};
				c.broadcast("status", c.vars.status);
			} catch (error) {
				const errorMessage =
					error instanceof Error ? error.message : "Unknown error";
				assistantMessage.content =
					assistantMessage.content ||
					"I hit a snag while responding. Please try again.";
				await saveMessage(c.db, assistantMessage);

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
			}
		},
	},
});

export const agentManager = actor({
	// The agent list is saved in SQLite, which survives restarts.
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
			await client.agent.getOrCreate([info.id]).getStatus();

			return info;
		},
		listAgents: (c) =>
			c.db.execute<AgentInfo>(
				"SELECT id, name, created_at AS createdAt FROM agents ORDER BY seq",
			),
	},
});

export const registry = setup({
	use: { agent, agentManager, codingSandbox },
});

registry.start();
