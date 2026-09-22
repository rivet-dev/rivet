import { createClient } from "rivetkit/client";
import type { registry } from "./server";

const client = createClient<typeof registry>("http://localhost:6420");
const agent = client.agent.getOrCreate(["support", "customer-123"]);

const conn = agent.connect();

await conn.prompt("Inspect the project and summarize its test failures.");
console.log(await conn.getLastAssistantText());

await conn.dispose();
