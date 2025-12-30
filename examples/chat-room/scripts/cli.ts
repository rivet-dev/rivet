import prompts from "prompts";
import { createClient } from "rivetkit/client";
import type { registry } from "../src/actors";

async function main() {
	const { username, room } = await initPrompt();

	// Create type-aware client
	const client = createClient<typeof registry>("http://localhost:3000/api/rivet");

	// connect to chat room
	const chatRoom = client.chatRoom.getOrCreate([room]).connect();

	// fetch history
	const history = await chatRoom.getHistory();
	console.log(
		`History:\n${history.map((m) => `[${m.sender}] ${m.text}`).join("\n")}`,
	);

	// listen for new messages
	let needsNewLine = false;
	chatRoom.on("newMessage", (message: any) => {
		if (needsNewLine) {
			needsNewLine = false;
			console.log();
		}
		console.log(`[${message.sender}] ${message.text}`);
	});

	// loop to send messages
	while (true) {
		needsNewLine = true;
		const message = await textPrompt("Message");
		if (!message) break;
		needsNewLine = false;
		await chatRoom.sendMessage(username, message);
	}

	await chatRoom.dispose();
}

async function initPrompt(): Promise<{
	room: string;
	username: string;
}> {
	return await prompts([
		{
			type: "text",
			name: "username",
			message: "Username",
		},
		{
			type: "text",
			name: "room",
			message: "Room",
		},
	]);
}

async function textPrompt(message: string): Promise<string> {
	const { x } = await prompts({
		type: "text",
		name: "x",
		message,
	});
	return x;
}

main();
