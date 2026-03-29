// Execute commands and manage processes inside the VM.

import { createClient } from "rivetkit/client";
import type { registry } from "./server.ts";

const client = createClient<typeof registry>("http://localhost:6420");
const agent = client.vm.getOrCreate(["my-agent"]);

// Run shell commands with exec()
const result = (await agent.exec("echo 'hello from shell'")) as {
	stdout: string;
	stderr: string;
	exitCode: number;
};
console.log("exec stdout:", result.stdout.trim());
console.log("exec exit code:", result.exitCode);

// Shell pipeline
const piped = (await agent.exec("echo hello | tr a-z A-Z")) as {
	stdout: string;
};
console.log("piped:", piped.stdout.trim());

// grep
await agent.writeFile("/tmp/data.txt", "apple\nbanana\ncherry\napricot\n");
const grepped = (await agent.exec("grep ap /tmp/data.txt")) as {
	stdout: string;
};
console.log("grep:", grepped.stdout.trim());

// sed
const sedResult = (await agent.exec(
	"echo 'hello world' | sed 's/world/agentOS/'",
)) as { stdout: string };
console.log("sed:", sedResult.stdout.trim());

// Spawn a Node.js script and wait for it to complete
await agent.writeFile(
	"/tmp/counter.mjs",
	`
let i = 0;
const interval = setInterval(() => {
  console.log("tick " + i++);
  if (i >= 3) { clearInterval(interval); }
}, 100);
`,
);

const proc = (await agent.spawn("node", ["/tmp/counter.mjs"])) as {
	pid: number;
};
console.log("Spawned process:", proc.pid);

// Wait for it to finish
const exitCode = (await agent.waitProcess(proc.pid)) as number;
console.log("Process exited with code:", exitCode);

// List all processes
const processes = await agent.listProcesses();
console.log("Processes:", JSON.stringify(processes));
