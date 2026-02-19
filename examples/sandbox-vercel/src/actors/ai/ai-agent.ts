import { openai } from "@ai-sdk/openai";
import { generateText, tool } from "ai";
import { actor, event } from "rivetkit";
import { z } from "zod";
import { getWeather } from "./my-tools.ts";
import type { Message } from "./types.ts";

export const aiAgent = actor({
	// Persistent state that survives restarts: https://rivet.dev/docs/actors/state
	state: {
		messages: [] as Message[],
	},
	events: {
		messageReceived: event<Message>(),
	},

	actions: {
		// Callable functions from clients: https://rivet.dev/docs/actors/actions
		getMessages: (c) => c.state.messages,

		sendMessage: async (c, userMessage: string) => {
			const userMsg: Message = {
				role: "user",
				content: userMessage,
				timestamp: Date.now(),
			};
			// State changes are automatically persisted
			c.state.messages.push(userMsg);

			const { text } = await generateText({
				model: openai("gpt-4o-mini"),
				prompt: userMessage,
				messages: c.state.messages,
				tools: {
					weather: tool({
						description: "Get the weather in a location",
						parameters: z.object({
							location: z
								.string()
								.describe(
									"The location to get the weather for",
								),
						}),
						execute: async ({ location }) => {
							return await getWeather(location);
						},
					}),
				},
			});

			const assistantMsg: Message = {
				role: "assistant",
				content: text,
				timestamp: Date.now(),
			};
			c.state.messages.push(assistantMsg);

			// Send events to all connected clients: https://rivet.dev/docs/actors/events
			c.broadcast("messageReceived", assistantMsg);

			return assistantMsg;
		},
	},
});
