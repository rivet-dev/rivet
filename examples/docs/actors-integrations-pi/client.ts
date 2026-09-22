import { createClient } from "rivetkit/client";
import type { registry } from "./server";

const client = createClient<typeof registry>("http://localhost:6420");
const agent = client.agent.getOrCreate(["support", "customer-123"]);

const conn = agent.connect();
conn.on("event", (event) => {
	if (
		event.type === "message_update" &&
		event.assistantMessageEvent.type === "text_delta"
	) {
		process.stdout.write(event.assistantMessageEvent.delta);
	}
});

await conn.prompt("Inspect the project and summarize its test failures.");
await conn.waitForIdle();

console.log(await conn.getMessages());
await conn.dispose();
