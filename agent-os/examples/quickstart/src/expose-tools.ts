import { AgentOs } from "@rivet-dev/agent-os-core";

const os = await AgentOs.create();
const session = await os.createSession("pi", {
  mcpServers: [
    { type: "local", command: "npx", args: ["my-tools-server"] },
    { type: "remote", url: "https://tools.example.com/mcp" },
  ]
});
