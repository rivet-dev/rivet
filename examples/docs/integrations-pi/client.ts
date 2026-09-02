import { createClient } from "rivetkit/client";
import type { registry } from "./server.js";

const client = createClient<typeof registry>("http://localhost:6420");
const agent = client.myAgent.getOrCreate(["support", "customer-123"]);

await agent.prompt("Inspect the project and summarize its test failures.");
const session = await agent.getSession();
const messages = await agent.getMessages();

console.log({ session, messages });
