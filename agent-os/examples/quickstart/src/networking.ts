import { AgentOs } from "@rivet-dev/agent-os-core";

const os = await AgentOs.create();

// Start a Node server inside the VM
os.spawn("node", ["server.js"]);

// Call it from the host
const res = await os.fetch(3000, new Request("http://localhost/"));
console.log(await res.text());
