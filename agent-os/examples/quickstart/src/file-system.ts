import { AgentOs } from "@rivet-dev/agent-os-core";

const os = await AgentOs.create({
  mounts: [{ type: "s3", path: "/data", bucket: "my-bucket" }]
});

await os.writeFile("/workspace/hello.txt", "Hello, world!");
const content = await os.readFile("/workspace/hello.txt");
const files = await os.readdir("/workspace");
