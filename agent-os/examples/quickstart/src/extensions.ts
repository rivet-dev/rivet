import { AgentOs } from "@rivet-dev/agent-os-core";

const os = await AgentOs.create({
  mounts: [{ type: "host", path: "/project", hostPath: "/home/user/app" }]
});

const session = await os.createSession("pi", {
  mcpServers: [{ type: "local", command: "npx", args: ["@playwright/mcp"] }],
  cwd: "/project"
});
