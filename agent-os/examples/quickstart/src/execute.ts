// Execute commands and manage processes inside the VM.
//
// exec() requires WASM shell binaries. Set WASM_COMMANDS_DIR to the
// directory containing them (e.g., secure-exec's compiled commands).
// spawn() works without WASM for Node.js scripts.

import { AgentOs } from "@rivet-dev/agent-os-core";

const commandDirs = process.env.WASM_COMMANDS_DIR
	? [process.env.WASM_COMMANDS_DIR]
	: undefined;

const vm = await AgentOs.create({ commandDirs });

// --- exec(): run shell commands (requires WASM commands) ---
if (commandDirs) {
	const result = await vm.exec("echo 'hello from shell'");
	console.log("exec stdout:", result.stdout.trim());
	console.log("exec exit code:", result.exitCode);

	// Shell pipeline
	const piped = await vm.exec("echo hello | tr a-z A-Z");
	console.log("piped:", piped.stdout.trim());
} else {
	console.log(
		"(skipping exec — set WASM_COMMANDS_DIR to enable shell commands)",
	);
}

// --- spawn(): run Node.js scripts directly (no WASM needed) ---
await vm.writeFile(
	"/tmp/greet.mjs",
	'console.log("Hello, " + process.env.NAME);',
);
const greet = vm.spawn("node", ["/tmp/greet.mjs"], {
	env: { NAME: "AgentOS" },
	onStdout: (data: Uint8Array) => {
		process.stdout.write(`spawn stdout: ${new TextDecoder().decode(data)}`);
	},
});
await greet.wait();

// Long-running process with streaming output
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
		process.stdout.write(`[counter] ${new TextDecoder().decode(data)}`);
	},
});
await proc.wait();

// Process management
console.log("\nProcesses:", vm.listProcesses());

await vm.dispose();
