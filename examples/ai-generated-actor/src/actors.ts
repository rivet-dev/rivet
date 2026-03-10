import { openai } from "@ai-sdk/openai";
import { streamText, type CoreMessage } from "ai";
import { actor, event, queue, setup } from "rivetkit";
import { dynamicActor } from "rivetkit/dynamic";

// Shared types for client usage
export type ChatMessage = {
	id: string;
	role: "user" | "assistant";
	content: string;
	createdAt: number;
};

export type ResponseEvent = {
	messageId: string;
	delta: string;
	content: string;
	done: boolean;
	error?: string;
};

export type CodeUpdateEvent = {
	code: string;
	revision: number;
};

export type CodeAgentState = {
	messages: ChatMessage[];
	code: string;
	codeRevision: number;
	status: "idle" | "thinking" | "error";
};

export const DEFAULT_ACTOR_CODE = `import { actor, event } from "rivetkit";

export default actor({
	state: {
		count: 0,
	},
	events: {
		countChanged: event<number>(),
	},
	actions: {
		increment: (c, amount = 1) => {
			c.state.count += amount;
			c.broadcast("countChanged", c.state.count);
			return c.state.count;
		},
		decrement: (c, amount = 1) => {
			c.state.count -= amount;
			c.broadcast("countChanged", c.state.count);
			return c.state.count;
		},
		getCount: (c) => c.state.count,
		reset: (c) => {
			c.state.count = 0;
			c.broadcast("countChanged", 0);
			return 0;
		},
	},
});
`;

const SYSTEM_PROMPT = `You are an AI assistant that generates RivetKit actor code. When the user describes what they want their actor to do, generate a complete actor module.

IMPORTANT: Your response MUST include a complete actor code block wrapped in \`\`\`typescript fences. Always provide the FULL actor code, not partial changes.

## RivetKit Actor Format

Every actor module must be a valid ESM module that default-exports an actor definition:

\`\`\`typescript
import { actor, event, queue } from "rivetkit";

export default actor({
  // Persistent state that survives restarts
  state: {
    count: 0,
    items: [] as string[],
  },

  // Events broadcast to connected clients (optional)
  events: {
    updated: event<{ count: number }>(),
  },

  // Queues for async message processing (optional)
  queues: {
    tasks: queue<{ type: string }>(),
  },

  // Actions callable by clients (RPC-style)
  actions: {
    increment: (c, amount: number = 1) => {
      c.state.count += amount;
      c.broadcast("updated", { count: c.state.count });
      return c.state.count;
    },
    getItems: (c) => c.state.items,
  },

  // Optional long-running background loop
  run: async (c) => {
    for await (const msg of c.queue.iter()) {
      // Process queue messages. msg.body has the payload.
    }
  },
});
\`\`\`

## Context API (the "c" parameter)

### State
- \`c.state\` - Read/write persistent state. Must be JSON-serializable.

### Broadcasting Events
- \`c.broadcast("eventName", payload)\` - Send event to all connected clients.

### Key-Value Storage
- \`await c.kv.get(key)\` - Get value by key (returns entry with .value)
- \`await c.kv.put(key, value)\` - Store value
- \`await c.kv.delete(key)\` - Delete key
- \`await c.kv.list({ prefix })\` - List entries by prefix

### Scheduling
- \`c.schedule.after(delayMs, "actionName", ...args)\` - Call action after delay
- \`c.schedule.at(timestampMs, "actionName", ...args)\` - Call action at specific time

### Actor Identity
- \`c.key\` - The actor's key array (e.g. ["my-key"])

### Destroying
- \`c.destroy()\` - Permanently delete the actor and its state

## Rules
1. Module MUST use \`export default actor({...})\` syntax
2. State must be JSON-serializable (no functions, Map, Set, Date objects)
3. Actions return JSON-serializable values
4. Only import from "rivetkit" (actor, event, queue are available)
5. Action handlers receive context \`c\` as first param, then user args
6. Keep code concise and well-structured
7. Include events when the actor has state that changes, so clients can subscribe

## Examples

### Todo List
\`\`\`typescript
import { actor, event } from "rivetkit";

export default actor({
  state: {
    todos: [] as { id: string; text: string; done: boolean }[],
  },
  events: {
    todosChanged: event<{ id: string; text: string; done: boolean }[]>(),
  },
  actions: {
    add: (c, text: string) => {
      const todo = { id: String(Date.now()), text, done: false };
      c.state.todos.push(todo);
      c.broadcast("todosChanged", c.state.todos);
      return todo;
    },
    toggle: (c, id: string) => {
      const todo = c.state.todos.find(t => t.id === id);
      if (todo) todo.done = !todo.done;
      c.broadcast("todosChanged", c.state.todos);
      return todo;
    },
    remove: (c, id: string) => {
      c.state.todos = c.state.todos.filter(t => t.id !== id);
      c.broadcast("todosChanged", c.state.todos);
    },
    list: (c) => c.state.todos,
  },
});
\`\`\`

### Rate Limiter
\`\`\`typescript
import { actor } from "rivetkit";

export default actor({
  state: {
    requests: [] as number[],
    limit: 10,
    windowMs: 60000,
  },
  actions: {
    check: (c) => {
      const now = Date.now();
      c.state.requests = c.state.requests.filter(t => now - t < c.state.windowMs);
      if (c.state.requests.length >= c.state.limit) {
        return { allowed: false, remaining: 0 };
      }
      c.state.requests.push(now);
      return { allowed: true, remaining: c.state.limit - c.state.requests.length };
    },
    setLimit: (c, limit: number, windowMs?: number) => {
      c.state.limit = limit;
      if (windowMs) c.state.windowMs = windowMs;
    },
  },
});
\`\`\`
`;

const buildId = (prefix: string) =>
	`${prefix}-${Date.now()}-${Math.random().toString(16).slice(2)}`;

// The code agent stores chat history and generated code, streaming AI responses
// via events as they arrive.
export const codeAgent = actor({
	// Persistent state: https://rivet.dev/docs/actors/state
	state: {
		messages: [] as ChatMessage[],
		code: DEFAULT_ACTOR_CODE,
		codeRevision: 1,
		status: "idle" as "idle" | "thinking" | "error",
	},
	queues: {
		chat: queue<{ text: string }>(),
	},
	events: {
		response: event<ResponseEvent>(),
		codeUpdated: event<CodeUpdateEvent>(),
		statusChanged: event<string>(),
	},

	// The run hook processes chat messages from the queue.
	run: async (c: any) => {
		for await (const queued of c.queue.iter()) {
			const { body } = queued;
			if (!body?.text || typeof body.text !== "string") {
				continue;
			}

			const userMessage: ChatMessage = {
				id: buildId("user"),
				role: "user",
				content: body.text.trim(),
				createdAt: Date.now(),
			};

			c.state.messages.push(userMessage);

			const assistantMessage: ChatMessage = {
				id: buildId("assistant"),
				role: "assistant",
				content: "",
				createdAt: Date.now(),
			};

			c.state.messages.push(assistantMessage);

			c.state.status = "thinking";
			c.broadcast("statusChanged", "thinking");

			const promptMessages: CoreMessage[] = [
				{ role: "system", content: SYSTEM_PROMPT },
				...c.state.messages
					.filter((m: ChatMessage) => m.content)
					.map((m: ChatMessage) => ({
						role: m.role as "user" | "assistant",
						content: m.content,
					})),
			];

			try {
				const result = await streamText({
					model: openai("gpt-5"),
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

				// Extract code from the response
				const codeMatch = content.match(
					/```(?:typescript|ts|javascript|js)?\n([\s\S]*?)```/,
				);
				if (codeMatch) {
					c.state.code = codeMatch[1].trim();
					c.state.codeRevision += 1;
					c.broadcast("codeUpdated", {
						code: c.state.code,
						revision: c.state.codeRevision,
					});
				}

				c.broadcast("response", {
					messageId: assistantMessage.id,
					delta: "",
					content: assistantMessage.content,
					done: true,
				});

				c.state.status = "idle";
				c.broadcast("statusChanged", "idle");
			} catch (error) {
				const errorMessage =
					error instanceof Error ? error.message : "Unknown error";

				assistantMessage.content =
					assistantMessage.content ||
					"Something went wrong while generating a response.";

				c.state.status = "error";

				c.broadcast("response", {
					messageId: assistantMessage.id,
					delta: "",
					content: assistantMessage.content,
					done: true,
					error: errorMessage,
				});
				c.broadcast("statusChanged", "error");
			}
		}
	},

	actions: {
		// Callable functions from clients: https://rivet.dev/docs/actors/actions
		getState: (c: any): CodeAgentState => ({
			messages: c.state.messages,
			code: c.state.code,
			codeRevision: c.state.codeRevision,
			status: c.state.status,
		}),
	},
});

// The dynamic actor loads its code from the codeAgent with the matching key.
const dynamicRunner = dynamicActor({
	load: async (c: any) => {
		const state = await c
			.client()
			.codeAgent.getOrCreate(c.key)
			.getState();

		return {
			source: state.code,
			nodeProcess: {
				memoryLimit: 256,
				cpuTimeLimitMs: 10_000,
			},
		};
	},
});

// Register actors for use: https://rivet.dev/docs/setup
export const registry = setup({
	use: { codeAgent, dynamicRunner },
});
