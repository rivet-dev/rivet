import { openai } from "@ai-sdk/openai";
import { streamText, type CoreMessage } from "ai";
import { actor, event, queue, setup } from "rivetkit";

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

export type AgentQueueMessage = {
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

const SYSTEM_PROMPT =
	"You are a focused AI assistant. Keep responses concise, actionable, and ready for handoff.";

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

export const agent = actor({
	// Persistent state that survives restarts: https://rivet.dev/docs/actors/state
	state: {
		messages: [] as AgentMessage[],
		status: {
			state: "idle",
			updatedAt: Date.now(),
		} as AgentStatus,
	},
	queues: {
		message: queue<AgentQueueMessage>(),
	},
	events: {
		messageAdded: event<AgentMessage>(),
		status: event<AgentStatus>(),
		response: event<AgentResponseEvent>(),
	},

	// The run hook keeps the agent listening for queued messages.
	run: async (c) => {
		for await (const queued of c.queue.iter()) {
			const { body } = queued;
			if (!body?.text || typeof body.text !== "string") {
				continue;
			}

			const sender = body.sender?.trim() || "Operator";
			const userMessage: AgentMessage = {
				id: buildId("user"),
				role: "user",
				sender,
				content: body.text.trim(),
				createdAt: Date.now(),
			};

			c.state.messages.push(userMessage);
			c.broadcast("messageAdded", userMessage);

			const promptMessages = buildPromptMessages(c.state.messages);

			const assistantMessage: AgentMessage = {
				id: buildId("assistant"),
				role: "assistant",
				sender: "Agent",
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
				const result = await streamText({
					model: openai("gpt-4o-mini"),
					messages: promptMessages,
				});

				let content = "";
				for await (const delta of result.textStream) {
					if (c.aborted) {
						break;
					}

					content += delta;
					assistantMessage.content = content;
					c.broadcast("response", {
						messageId: assistantMessage.id,
						delta,
						content,
						done: false,
					});
				}

				assistantMessage.content = content || assistantMessage.content;
				c.broadcast("response", {
					messageId: assistantMessage.id,
					delta: "",
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
		}
	},

	actions: {
		// Callable functions from clients: https://rivet.dev/docs/actors/actions
		getHistory: (c) => c.state.messages,
		getStatus: (c) => c.state.status,
	},
});

export const agentManager = actor({
	// Persistent state that survives restarts: https://rivet.dev/docs/actors/state
	state: {
		agents: [] as AgentInfo[],
	},

	actions: {
		// Callable functions from clients: https://rivet.dev/docs/actors/actions
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
			const handle = client.agent.getOrCreate([info.id]);
			await handle.getStatus();

			return info;
		},

		listAgents: (c) => c.state.agents,
	},
});

// Register actors for use: https://rivet.dev/docs/setup
export const registry = setup({
	use: { agent, agentManager },
});
