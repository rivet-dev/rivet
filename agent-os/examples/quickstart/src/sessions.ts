import { AgentOs } from "@rivet-dev/agent-os-core";

const os = await AgentOs.create();
const session = await os.createSession("pi");

session.onSessionEvent((event) => {
  console.log(event);
});

await session.prompt("Write a Python script that calculates pi");
