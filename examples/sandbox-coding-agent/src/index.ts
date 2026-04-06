import fs from "node:fs";
import path from "node:path";
import { actor, event, setup } from "rivetkit";
import {
	daytona,
	docker,
	e2b,
	sandboxActor,
	type SessionEvent,
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

export const agent = actor({
	state: {
		messages: [] as AgentMessage[],
		status: {
			state: "idle",
			updatedAt: Date.now(),
		} as AgentStatus,
		sessionId: null as string | null,
	},
	events: {
		messageAdded: event<AgentMessage>(),
		status: event<AgentStatus>(),
		response: event<AgentResponseEvent>(),
	},
	actions: {
		getHistory: (c) => c.state.messages,
		getStatus: (c) => c.state.status,
		sendMessage: async (
			c,
			input: { text: string; sender?: string },
		) => {
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
			c.state.messages.push(userMessage);
			c.broadcast("messageAdded", userMessage);

			const assistantMessage: AgentMessage = {
				id: buildId("assistant"),
				role: "assistant",
				sender: "Sandbox Actor",
				content: "",
				createdAt: Date.now(),
			};
			c.state.messages.push(assistantMessage);
			c.broadcast("messageAdded", assistantMessage);

			c.state.status = {
				state: "thinking",
				updatedAt: Date.now(),
			};
			c.broadcast("status", c.state.status);

			try {
				const sandbox = c.client<typeof registry>().codingSandbox.getOrCreate([
					c.key[0],
				]);
				const sessionId = c.state.sessionId ?? buildId("session");

				if (!c.state.sessionId) {
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
					c.state.sessionId = sessionId;
				}

				const previousEvents = await fetchAllSessionEvents(sandbox, sessionId);
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

				const nextEvents = await fetchAllSessionEvents(sandbox, sessionId);
				const assistantText = extractAssistantText(
					nextEvents,
					previousMaxEventIndex,
				);
				assistantMessage.content =
					assistantText ||
					`Prompt completed with stop reason: ${promptResult.response?.stopReason ?? "unknown"}`;

				c.broadcast("response", {
					messageId: assistantMessage.id,
					delta: assistantMessage.content,
					content: assistantMessage.content,
					done: true,
				});

				c.state.status = {
					state: "idle",
					updatedAt: Date.now(),
				};
				c.broadcast("status", c.state.status);
			} catch (error) {
				const errorMessage =
					error instanceof Error ? error.message : "Unknown error";
				assistantMessage.content =
					assistantMessage.content ||
					"I hit a snag while responding. Please try again.";

				c.state.status = {
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
				c.broadcast("status", c.state.status);
			}
		},
	},
});

export const agentManager = actor({
	state: {
		agents: [] as AgentInfo[],
	},
	actions: {
		createAgent: async (c, name?: string) => {
			const trimmedName = name?.trim();
			const agentName =
				trimmedName || `Agent ${c.state.agents.length + 1}`;
			const info: AgentInfo = {
				id: buildId("agent"),
				name: agentName,
				createdAt: Date.now(),
			};

			c.state.agents.push(info);

			const client = c.client<typeof registry>();
			await client.agent.getOrCreate([info.id]).getStatus();

			return info;
		},
		listAgents: (c) => c.state.agents,
	},
});

export const registry = setup({
	use: { agent, agentManager, codingSandbox },
});

registry.start();
