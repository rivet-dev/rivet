// Minimal agentOS example: create a VM, write a file, read it back.

import { createClient } from "rivetkit/client";
import type { registry } from "./server.ts";

const client = createClient<typeof registry>("http://localhost:6420");
const agent = client.vm.getOrCreate(["my-agent"]);

await agent.writeFile("/hello.txt", "Hello from agentOS!");
const content = (await agent.readFile("/hello.txt")) as Uint8Array;
console.log(new TextDecoder().decode(content));
