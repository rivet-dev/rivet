<<<<<<< HEAD
// Execute commands and manage processes inside the VM.

import { AgentOs } from "@rivet-dev/agent-os-core";
import common from "@rivet-dev/agent-os-common";

const vm = await AgentOs.create({ software: [common] });

// Run shell commands with exec()
const result = await vm.exec("echo 'hello from shell'");
console.log("exec stdout:", result.stdout.trim());
console.log("exec exit code:", result.exitCode);

// Shell pipeline
const piped = await vm.exec("echo hello | tr a-z A-Z");
console.log("piped:", piped.stdout.trim());

// grep
await vm.writeFile("/tmp/data.txt", "apple\nbanana\ncherry\napricot\n");
const grepped = await vm.exec("grep ap /tmp/data.txt");
console.log("grep:", grepped.stdout.trim());

// sed
const sedResult = await vm.exec("echo 'hello world' | sed 's/world/agentOS/'");
console.log("sed:", sedResult.stdout.trim());

// Spawn a Node.js script and wait for it to complete
await vm.writeFile(
	"/tmp/counter.mjs",
	`
let i = 0;
const interval = setInterval(() => {
  console.log("tick " + i++);
  if (i >= 3) { clearInterval(interval); }
}, 100);
`,
);

const proc = vm.spawn("node", ["/tmp/counter.mjs"], {
	onStdout: (data: Uint8Array) => {
		process.stdout.write(`[process ${proc.pid}] ${new TextDecoder().decode(data)}`);
	},
});
console.log("Spawned process:", proc.pid);

// Wait for it to finish
const exitCode = await vm.waitProcess(proc.pid);
console.log("Process exited with code:", exitCode);

// List all processes
console.log("Processes:", vm.listProcesses());

await vm.dispose();
=======
import { AgentOs } from "@rivet-dev/agent-os-core";

const os = await AgentOs.create();

const proc = os.spawn("python3", ["train.py"]);
const result = await os.exec("ls -la /workspace");

const tree = os.processTree();
os.stopProcess(proc.pid);
>>>>>>> 84b3b6377 (feat(website): fix agent logo positioning in hero squircle)
