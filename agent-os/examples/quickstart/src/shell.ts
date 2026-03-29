// Interactive shell via openShell() with PTY support.
//
// openShell() allocates a PTY and spawns a shell process. This is meant
// for interactive terminal sessions. For scripted command execution,
// prefer exec() which is simpler and captures stdout/stderr directly.
//
// Requires WASM shell binaries. Set WASM_COMMANDS_DIR to the directory
// containing them (e.g., secure-exec's compiled commands).

import { AgentOs } from "@rivet-dev/agent-os-core";

const commandDirs = process.env.WASM_COMMANDS_DIR
	? [process.env.WASM_COMMANDS_DIR]
	: undefined;

if (!commandDirs) {
	console.log("Set WASM_COMMANDS_DIR to run this example.");
} else {
	const vm = await AgentOs.create({ commandDirs });

	// For scripted shell commands, exec() is the easiest approach
	const result = await vm.exec("echo 'hello from shell' && pwd");
	console.log("exec output:", result.stdout.trim());

	// openShell() returns a shell ID for interactive use.
	// It's best suited for connecting to a real terminal (e.g., via
	// kernel.connectTerminal()), not for scripted I/O.
	const { shellId } = vm.openShell();

	let output = "";
	vm.onShellData(shellId, (data: Uint8Array) => {
		const text = new TextDecoder().decode(data);
		// Filter WASM shell noise (history warnings, ANSI escapes)
		for (const line of text.split("\n")) {
			if (line.includes("WARN") || line.trim() === "") continue;
			output += `${line}\n`;
		}
	});

	// Send a command and wait for output
	await new Promise((r) => setTimeout(r, 500));
	vm.writeShell(shellId, "echo PTY_WORKS\n");
	await new Promise((r) => setTimeout(r, 500));

	vm.closeShell(shellId);

	console.log("PTY output:", output.trim());

	await vm.dispose();
}
