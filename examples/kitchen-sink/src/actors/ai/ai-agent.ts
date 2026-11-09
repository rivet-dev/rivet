import { openai } from "@ai-sdk/openai";
import { generateText, tool } from "ai";
import { actor, event } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";
import { z } from "zod";
import { getWeather } from "./my-tools.ts";
import type { Message } from "./types.ts";

export const aiAgent = actor({
	// Messages are stored in SQLite so they survive restarts: https://rivet.dev/docs/actors/sqlite
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS messages (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					role TEXT NOT NULL,
					content TEXT NOT NULL,
					timestamp INTEGER NOT NULL
				);
			`);
		},
	}),
	events: {
		messageReceived: event<Message>(),
	},

	actions: {
		// Callable functions from clients: https://rivet.dev/docs/actors/actions
		getMessages: (c) => loadMessages(c.db),

		sendMessage: async (c, userMessage: string) => {
			const userMsg: Message = {
				role: "user",
				content: userMessage,
				timestamp: Date.now(),
			};
			await saveMessage(c.db, userMsg);
			const messages = await loadMessages(c.db);

			const { text } = await generateText({
				model: openai("gpt-4o-mini"),
				prompt: userMessage,
				messages,
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
			await saveMessage(c.db, assistantMsg);

			// Send events to all connected clients: https://rivet.dev/docs/actors/events
			c.broadcast("messageReceived", assistantMsg);

			return assistantMsg;
		},
	},
});

async function loadMessages(database: RawAccess): Promise<Message[]> {
	return await database.execute<Message>(
		"SELECT role, content, timestamp FROM messages ORDER BY id",
	);
}

async function saveMessage(database: RawAccess, message: Message): Promise<void> {
	await database.execute(
		"INSERT INTO messages (role, content, timestamp) VALUES (?, ?, ?)",
		message.role,
		message.content,
		message.timestamp,
	);
}
