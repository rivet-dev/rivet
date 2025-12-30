#!/usr/bin/env tsx

import { createClient } from "rivetkit/client";
import type { registry } from "./src/actors";

async function main() {
	console.log("=== Chat Room E2E Test ===\n");

	// Create client pointing to serverless handler
	const client = createClient<typeof registry>("http://localhost:3000/api/rivet");

	// Get or create a test room
	console.log("1. Getting/creating chat room 'test-e2e-room'...");
	const chatRoom = client.chatRoom.getOrCreate(["test-e2e-room"]);

	// Fetch initial history
	console.log("2. Fetching message history...");
	const initialHistory = await chatRoom.getHistory();
	console.log(`   Initial history has ${initialHistory.length} messages`);

	// Send a message
	console.log("3. Sending message...");
	const message1 = await chatRoom.sendMessage("TestUser", "Hello from e2e test!");
	console.log(`   Sent: [${message1.sender}] ${message1.text} (ts: ${message1.timestamp})`);

	// Send another message
	console.log("4. Sending second message...");
	const message2 = await chatRoom.sendMessage("TestUser2", "Reply from test!");
	console.log(`   Sent: [${message2.sender}] ${message2.text} (ts: ${message2.timestamp})`);

	// Get updated history
	console.log("5. Fetching updated history...");
	const updatedHistory = await chatRoom.getHistory();
	console.log(`   Updated history has ${updatedHistory.length} messages:`);
	for (const msg of updatedHistory) {
		console.log(`   - [${msg.sender}] ${msg.text}`);
	}

	console.log("\n✅ E2E test completed successfully!");
}

main().catch((err) => {
	console.error("❌ E2E test failed:", err);
	process.exit(1);
});
