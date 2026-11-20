import { openai } from "@ai-sdk/openai";
import { streamText } from "ai";
import { actor, event } from "rivetkit";
import { db } from "rivetkit/db";

type ChatMessage = {
	role: "user" | "assistant";
	content: string;
};

export const chatAgent = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
        CREATE TABLE IF NOT EXISTS messages (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          role TEXT NOT NULL,
          content TEXT NOT NULL
        )
      `);
		},
	}),

	events: {
		// replyId tells apart replies that stream at the same time
		token: event<{ replyId: string; text: string }>(),
		reply: event<{ replyId: string; text: string }>(),
	},

	actions: {
		history: (c) =>
			c.db.execute<ChatMessage>(
				"SELECT role, content FROM messages ORDER BY id",
			),

		send: async (c, text: string) => {
			await c.db.execute(
				"INSERT INTO messages (role, content) VALUES ('user', ?)",
				text,
			);
			const messages = await c.db.execute<ChatMessage>(
				"SELECT role, content FROM messages ORDER BY id",
			);

			// Any AI SDK provider works here
			const result = streamText({
				model: openai("gpt-5"),
				messages,
				abortSignal: c.abortSignal,
			});

			// Stream tokens to every connected client as they arrive
			const replyId = crypto.randomUUID();
			let reply = "";
			for await (const chunk of result.textStream) {
				reply += chunk;
				c.broadcast("token", { replyId, text: chunk });
			}

			await c.db.execute(
				"INSERT INTO messages (role, content) VALUES ('assistant', ?)",
				reply,
			);
			c.broadcast("reply", { replyId, text: reply });
			return reply;
		},
	},
});
