import { createClient } from "rivetkit/client";
import type { registry } from "./index";

// Connect to the server from index.ts. The registry type checks every call.
const client = createClient<typeof registry>("http://localhost:6420");

// Get the room keyed "general", creating it on first use.
const room = client.chatRoom.getOrCreate(["general"]);

// Open a realtime connection and print each message as it arrives.
const connection = room.connect();
connection.on("newMessage", (message: { sender: string; text: string }) => {
	console.log(`${message.sender}: ${message.text}`);
});

// Send a message. The Actor saves it to SQLite before broadcasting it.
await connection.sendMessage("alice", "Hello, Rivet!");

// The history is still there after the Actor sleeps or restarts.
console.log(await room.getHistory());
