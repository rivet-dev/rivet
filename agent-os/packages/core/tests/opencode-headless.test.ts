import { resolve } from "node:path";
import { afterEach, beforeEach, describe, expect, test } from "vitest";
import { AgentOs } from "../src/index.js";

/**
 * Use the workspace root as module access CWD. With shamefully-hoist=true
 * in .npmrc, all transitive dependencies are hoisted to the root node_modules,
 * making them accessible via the ModuleAccessFileSystem overlay.
 */
const MODULE_ACCESS_CWD = resolve(import.meta.dirname, "..");

describe.skip("OpenCode headless inside VM", () => {
	let vm: AgentOs;

	beforeEach(async () => {
		vm = await AgentOs.create({
			moduleAccessCwd: MODULE_ACCESS_CWD,
		});
	});

	afterEach(async () => {
		await vm.dispose();
	});

	test("opencode-ai package is mounted in VM via ModuleAccessFileSystem", async () => {
		// Verify the opencode-ai package.json is readable from inside the VM
		// through the ModuleAccessFileSystem overlay at /root/node_modules/
		const script = `
const fs = require("fs");
const pkgPath = "/root/node_modules/opencode-ai/package.json";
const exists = fs.existsSync(pkgPath);
console.log("exists:" + exists);
if (exists) {
  const pkg = JSON.parse(fs.readFileSync(pkgPath, "utf-8"));
  console.log("name:" + pkg.name);
  console.log("version:" + pkg.version);
  console.log("bin:" + JSON.stringify(pkg.bin));
}
`;
		await vm.writeFile("/tmp/check-opencode.mjs", script);

		let stdout = "";
		let stderr = "";

		const { pid } = vm.spawn("node", ["/tmp/check-opencode.mjs"], {
			onStdout: (data: Uint8Array) => {
				stdout += new TextDecoder().decode(data);
			},
			onStderr: (data: Uint8Array) => {
				stderr += new TextDecoder().decode(data);
			},
		});

		const exitCode = await vm.waitProcess(pid);

		expect(exitCode, `Failed. stderr: ${stderr}`).toBe(0);
		expect(stdout).toContain("exists:true");
		expect(stdout).toContain("name:opencode-ai");
		expect(stdout).toContain('"opencode":"./bin/opencode"');
	}, 30_000);

	test("opencode wrapper script resolves platform binary path", async () => {
		// The bin/opencode wrapper is a Node.js CJS script that resolves the
		// platform-specific native binary. Test that the resolution logic works
		// inside the VM (reads os.platform/arch, finds binary in node_modules).
		const script = `
const fs = require("fs");
const path = require("path");
const os = require("os");

const platform = os.platform();
const arch = os.arch();
console.log("platform:" + platform);
console.log("arch:" + arch);

// Check if the platform binary package is visible in the VM
const binaryPkgs = [
  "opencode-" + platform + "-" + arch,
  "opencode-" + platform + "-" + arch + "-baseline",
];
for (const pkg of binaryPkgs) {
  const binPath = "/root/node_modules/" + pkg + "/bin/opencode";
  const exists = fs.existsSync(binPath);
  console.log(pkg + ":" + exists);
}
`;
		await vm.writeFile("/tmp/check-binary.mjs", script);

		let stdout = "";
		let stderr = "";

		const { pid } = vm.spawn("node", ["/tmp/check-binary.mjs"], {
			onStdout: (data: Uint8Array) => {
				stdout += new TextDecoder().decode(data);
			},
			onStderr: (data: Uint8Array) => {
				stderr += new TextDecoder().decode(data);
			},
		});

		const exitCode = await vm.waitProcess(pid);

		expect(exitCode, `Failed. stderr: ${stderr}`).toBe(0);
		expect(stdout).toContain("platform:linux");
		expect(stdout).toContain("arch:x64");
		// At least one binary package should be visible
		expect(stdout).toMatch(/opencode-linux-x64(-baseline)?:true/);
	}, 30_000);

	test("opencode native binary cannot execute inside VM", async () => {
		// OpenCode is a compiled native ELF binary (not Node.js). The secure-exec
		// VM can only execute JavaScript and WASM — native binaries are not supported.
		// The kernel returns ENOENT because it can't resolve ELF binaries as commands.
		const script = `
const childProcess = require("child_process");
const fs = require("fs");

// Find the binary (same logic as the wrapper)
const candidates = [
  "/root/node_modules/opencode-linux-x64-baseline/bin/opencode",
  "/root/node_modules/opencode-linux-x64/bin/opencode",
];
let binaryPath = null;
for (const c of candidates) {
  if (fs.existsSync(c)) {
    binaryPath = c;
    break;
  }
}

if (!binaryPath) {
  console.log("result:no-binary-found");
  process.exit(0);
}

console.log("binary-found:" + binaryPath);

// Attempt to spawn the native binary — this should fail in the VM
try {
  const result = childProcess.spawnSync(binaryPath, ["--version"], { timeout: 5000 });
  console.log("status:" + result.status);
  // VM bridge returns {} for error (not a real Error), and Buffer for stdout/stderr
  if (result.stderr) {
    const errStr = Buffer.isBuffer(result.stderr) ? result.stderr.toString("utf-8") : String(result.stderr);
    console.log("stderr:" + errStr);
  }
} catch (e) {
  console.log("exception:" + e.message);
}
`;
		await vm.writeFile("/tmp/try-spawn.mjs", script);

		let stdout = "";
		let stderr = "";

		const { pid } = vm.spawn("node", ["/tmp/try-spawn.mjs"], {
			onStdout: (data: Uint8Array) => {
				stdout += new TextDecoder().decode(data);
			},
			onStderr: (data: Uint8Array) => {
				stderr += new TextDecoder().decode(data);
			},
		});

		const exitCode = await vm.waitProcess(pid);

		// The script should run (exit 0) — the error is captured internally
		expect(exitCode, `Script failed. stderr: ${stderr}`).toBe(0);
		// The binary path should be found (ModuleAccessFileSystem provides it)
		expect(stdout).toContain("binary-found:");
		// Kernel can't resolve native ELF binary as a command — returns ENOENT
		expect(stdout).toContain("status:1");
		expect(stdout).toContain("ENOENT");
	}, 30_000);
});
