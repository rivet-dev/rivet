import { AgentOs } from "@rivet-dev/agent-os-core";

const os = await AgentOs.create();
const session = await os.createSession("pi");

// Schedule a recurring task
setInterval(async () => {
  await session.prompt("Check for dependency updates and open PRs");
}, 6 * 60 * 60 * 1000); // Every 6 hours
