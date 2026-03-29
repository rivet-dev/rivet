// Minimal agentOS example: create a VM, write a file, read it back.

import { AgentOs } from "@rivet-dev/agent-os-core";

const vm = await AgentOs.create();

await vm.writeFile("/hello.txt", "Hello from agentOS!");
const content = await vm.readFile("/hello.txt");
console.log(new TextDecoder().decode(content));

await vm.dispose();
