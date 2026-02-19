import { SandboxAgent } from "sandbox-agent";
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

type ItemContentPart = {
	type?: string;
	text?: string;
};

type TranscriptItem = {
	item_id?: string;
	role?: string;
	content?: ItemContentPart[];
};

type ItemEventData = {
	item?: TranscriptItem;
};

type ItemDeltaEventData = {
	item_id?: string;
	delta?: string;
};

type StreamEvent = {
	type: string;
	data?: unknown;
};

const buildId = (prefix: string) =>
	`${prefix}-${Date.now()}-${Math.random().toString(16).slice(2)}`;

const createSandboxClient = async () => {
	const baseUrl = process.env.SANDBOX_AGENT_URL;
	if (!baseUrl && process.env.VERCEL) {
		throw new Error("SANDBOX_AGENT_URL is required when running on Vercel.");
	}
	if (baseUrl) {
		return SandboxAgent.connect({
			baseUrl,
			token: process.env.SANDBOX_AGENT_TOKEN,
		});
	}

	return SandboxAgent.start();
};

let sandboxClientPromise: ReturnType<typeof createSandboxClient> | null = null;

const getSandboxClient = () => {
	if (!sandboxClientPromise) {
		sandboxClientPromise = createSandboxClient();
	}

	return sandboxClientPromise;
};

const extractItem = (data: unknown): TranscriptItem | null => {
	if (!data || typeof data !== "object") {
		return null;
	}

	const item = (data as ItemEventData).item;
	if (!item || typeof item !== "object") {
		return null;
	}

	return item;
};

const extractDelta = (data: unknown): ItemDeltaEventData | null => {
	if (!data || typeof data !== "object") {
		return null;
	}

	const delta = data as ItemDeltaEventData;
	return delta;
};

const extractText = (item: TranscriptItem): string => {
	if (!Array.isArray(item.content)) {
		return "";
	}

	return item.content
		.map((part) => (part?.type === "text" ? part.text ?? "" : ""))
		.join("");
};

export const agent = actor({
	// Persistent state that survives restarts: https://rivet.dev/docs/actors/state
	state: {
		messages: [] as AgentMessage[],
		status: {
			state: "idle",
			updatedAt: Date.now(),
		} as AgentStatus,
		sessionId: null as string | null,
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

			const assistantMessage: AgentMessage = {
				id: buildId("assistant"),
				role: "assistant",
				sender: "Sandbox Agent",
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
				const client = await getSandboxClient();
				const sessionId = c.state.sessionId ?? buildId("session");
				if (!c.state.sessionId) {
					const agentId = process.env.SANDBOX_AGENT_AGENT ?? "codex";
					const permissionMode =
						process.env.SANDBOX_AGENT_PERMISSION_MODE ?? "bypass";

					await client.createSession(sessionId, {
						agent: agentId,
						agentMode: "code",
						permissionMode,
					});

					c.state.sessionId = sessionId;
				}

					const eventStream = await client.streamTurn(sessionId, {
						message: userMessage.content,
					});

				let content = "";
				let assistantItemId: string | null = null;

				for await (const rawEvent of eventStream) {
					const event = rawEvent as StreamEvent;
					if (c.aborted) {
						break;
					}

					if (event.type === "item.started") {
						const item = extractItem(event.data);
						if (item?.role === "assistant") {
							assistantItemId = item.item_id ?? null;
						}
					}

					if (event.type === "item.delta") {
						const deltaData = extractDelta(event.data);
						if (
							deltaData?.delta &&
							(!assistantItemId || deltaData.item_id === assistantItemId)
						) {
							content += deltaData.delta;
							assistantMessage.content = content;
							c.broadcast("response", {
								messageId: assistantMessage.id,
								delta: deltaData.delta,
								content,
								done: false,
							});
						}
					}

					if (event.type === "item.completed") {
						const item = extractItem(event.data);
						if (
							item &&
							item.role === "assistant" &&
							(!assistantItemId || item.item_id === assistantItemId)
						) {
							const finalText = extractText(item);
							if (finalText) {
								content = finalText;
							}
						}
					}
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
