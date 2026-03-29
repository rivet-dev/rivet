import { resolve } from "node:path";
import { afterEach, beforeEach, describe, expect, test } from "vitest";
import { AgentOs } from "../src/index.js";

/**
 * US-015: Investigate Claude Code SDK in secure-exec VM
 *
 * FINDINGS SUMMARY:
 * The @anthropic-ai/claude-code package is a ~13MB bundled ESM JavaScript file (cli.js).
 * Unlike OpenCode (native Go binary), Claude Code is pure JS. The ESM bundle can be
 * loaded (dynamic import succeeds) after secure-exec fixes, but the CLI cannot complete
 * startup because it depends on native vendor binaries and complex runtime infrastructure.
 *
 * Package characteristics:
 * - bin: { "claude": "cli.js" } — single bundled ESM entry point (~13MB)
 * - type: "module" — ESM format using import.meta.url + createRequire()
 * - No "exports" or "main" field — CLI-only package, no library API
 * - dependencies: {} — everything bundled into cli.js
 * - vendor/ripgrep/ — native ELF binary for code search (Grep tool)
 * - vendor/audio-capture/ — native .node addon for audio (voice features)
 * - Has built-in JSON-RPC / ACP support (speaks ACP natively like OpenCode)
 *
 * Secure-exec issues fixed during this investigation:
 * 1. ESM wrappers for deferred core modules (async_hooks, perf_hooks, worker_threads,
 *    diagnostics_channel, net, tls, readline) — previously only CJS require() worked
 * 2. ESM wrappers for path submodules (path/win32, path/posix, stream/consumers) —
 *    not in KNOWN_BUILTIN_MODULES set
 * 3. import.meta.url callback in V8 runtime — was not implemented, returned undefined
 * 4. General fallback for node:-prefixed builtins in loadFile handler
 *
 * Why Claude Code CLI cannot run inside the VM:
 * - Depends on native ripgrep binary for Grep tool (ENOENT in VM)
 * - Complex async startup sequence (config, auth, terminal) that hangs
 * - No TTY support (process.stdout.isTTY is false)
 * - 13MB bundle takes significant time to parse/evaluate
 *
 * CONCLUSION: The ESM bundle LOADS successfully but the CLI CANNOT complete startup.
 * US-016 through US-018 should be SKIPPED.
 */

const MODULE_ACCESS_CWD = resolve(import.meta.dirname, "..");

describe.skip("Claude Code SDK investigation", () => {
	let vm: AgentOs;

	beforeEach(async () => {
		vm = await AgentOs.create({
			moduleAccessCwd: MODULE_ACCESS_CWD,
		});
	});

	afterEach(async () => {
		await vm.dispose();
	});

	test("claude-code package is mounted in VM via ModuleAccessFileSystem", async () => {
		const script = `
const fs = require("fs");
const pkgPath = "/root/node_modules/@anthropic-ai/claude-code/package.json";
const exists = fs.existsSync(pkgPath);
console.log("exists:" + exists);
if (exists) {
  const pkg = JSON.parse(fs.readFileSync(pkgPath, "utf-8"));
  console.log("name:" + pkg.name);
  console.log("version:" + pkg.version);
  console.log("type:" + pkg.type);
  console.log("bin:" + JSON.stringify(pkg.bin));
}
`;
		await vm.writeFile("/tmp/check-claude-code.mjs", script);

		let stdout = "";
		let stderr = "";

		const { pid } = vm.spawn("node", ["/tmp/check-claude-code.mjs"], {
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
		expect(stdout).toContain("name:@anthropic-ai/claude-code");
		expect(stdout).toContain("type:module");
		expect(stdout).toContain('"claude":"cli.js"');
	}, 30_000);

	test("cli.js entry point is accessible and is ESM", async () => {
		const script = `
const fs = require("fs");
const cliPath = "/root/node_modules/@anthropic-ai/claude-code/cli.js";
const exists = fs.existsSync(cliPath);
console.log("cli-exists:" + exists);
if (exists) {
  const stat = fs.statSync(cliPath);
  console.log("size:" + stat.size);
  const fd = fs.openSync(cliPath, "r");
  const buf = Buffer.alloc(500);
  fs.readSync(fd, buf, 0, 500, 0);
  fs.closeSync(fd);
  const header = buf.toString("utf-8");
  console.log("is-esm:" + header.includes("import{"));
  console.log("has-shebang:" + header.startsWith("#!/usr/bin/env node"));
}
`;
		await vm.writeFile("/tmp/check-cli.mjs", script);

		let stdout = "";
		let stderr = "";

		const { pid } = vm.spawn("node", ["/tmp/check-cli.mjs"], {
			onStdout: (data: Uint8Array) => {
				stdout += new TextDecoder().decode(data);
			},
			onStderr: (data: Uint8Array) => {
				stderr += new TextDecoder().decode(data);
			},
		});

		const exitCode = await vm.waitProcess(pid);

		expect(exitCode, `Failed. stderr: ${stderr}`).toBe(0);
		expect(stdout).toContain("cli-exists:true");
		expect(stdout).toContain("is-esm:true");
		expect(stdout).toContain("has-shebang:true");
	}, 30_000);

	test("vendor ripgrep binary is accessible but cannot execute in VM", async () => {
		// Claude Code bundles native ripgrep (ELF) for code search.
		// The binary file is accessible via ModuleAccessFileSystem overlay
		// but cannot be spawned — kernel only supports JS/WASM commands.
		// Note: .node native addons (audio-capture) are blocked by the
		// overlay itself (ERR_MODULE_ACCESS_NATIVE_ADDON).
		const script = `
const fs = require("fs");
const childProcess = require("child_process");
const os = require("os");

const platform = os.platform();
const arch = os.arch();

const rgPath = "/root/node_modules/@anthropic-ai/claude-code/vendor/ripgrep/" + arch + "-" + platform + "/rg";
const rgExists = fs.existsSync(rgPath);
console.log("rg-exists:" + rgExists);

if (rgExists) {
  try {
    const result = childProcess.spawnSync(rgPath, ["--version"], { timeout: 5000 });
    console.log("rg-status:" + result.status);
    if (result.stderr) {
      const errStr = Buffer.isBuffer(result.stderr) ? result.stderr.toString("utf-8") : String(result.stderr);
      console.log("rg-stderr:" + errStr);
    }
  } catch (e) {
    console.log("rg-exception:" + e.message);
  }
}
`;
		await vm.writeFile("/tmp/check-vendor.mjs", script);

		let stdout = "";
		let stderr = "";

		const { pid } = vm.spawn("node", ["/tmp/check-vendor.mjs"], {
			onStdout: (data: Uint8Array) => {
				stdout += new TextDecoder().decode(data);
			},
			onStderr: (data: Uint8Array) => {
				stderr += new TextDecoder().decode(data);
			},
		});

		const exitCode = await vm.waitProcess(pid);

		expect(exitCode, `Failed. stderr: ${stderr}`).toBe(0);
		expect(stdout).toContain("rg-exists:true");
		// Ripgrep binary can't execute — kernel returns ENOENT or status 1
		expect(stdout).toMatch(/rg-status:1|rg-stderr:.*ENOENT/);
	}, 30_000);

	test("import.meta.url works correctly in VM ESM modules", async () => {
		// SECURE-EXEC FIX: Added HostInitializeImportMetaObjectCallback to V8 runtime
		// so import.meta.url returns a proper file: URL. Claude Code uses
		// createRequire(import.meta.url) which requires this to be a valid URL.
		const script = `
console.log("import.meta.url:" + import.meta.url);
console.log("typeof:" + typeof import.meta.url);
try {
  const { createRequire } = await import("node:module");
  const require = createRequire(import.meta.url);
  console.log("createRequire:success");
} catch (e) {
  console.log("createRequire-error:" + e.message);
}
`;
		await vm.writeFile("/tmp/test-meta.mjs", script);

		let stdout = "";

		const { pid } = vm.spawn("node", ["/tmp/test-meta.mjs"], {
			onStdout: (data: Uint8Array) => {
				stdout += new TextDecoder().decode(data);
			},
		});

		const exitCode = await vm.waitProcess(pid);

		expect(exitCode).toBe(0);
		expect(stdout).toContain("import.meta.url:file:///tmp/test-meta.mjs");
		expect(stdout).toContain("typeof:string");
		expect(stdout).toContain("createRequire:success");
	}, 30_000);

	test("cli.js ESM bundle loads via dynamic import", async () => {
		// SECURE-EXEC FIXES VERIFIED: After adding ESM wrappers for deferred
		// core modules (async_hooks, perf_hooks, etc.), path submodules
		// (path/win32, path/posix), stream/consumers, and the import.meta.url
		// callback, the 13MB ESM bundle loads successfully via dynamic import.
		//
		// However, the CLI startup hangs because it depends on:
		// - Native ripgrep binary (for Grep tool)
		// - Terminal/TTY features (process.stdout.isTTY)
		// - Complex async initialization (config, auth, network)
		const script = `
async function main() {
  try {
    console.log("attempting-import");
    const mod = await import("/root/node_modules/@anthropic-ai/claude-code/cli.js");
    console.log("import-success");
    console.log("exports:" + Object.keys(mod).join(","));
  } catch (e) {
    console.log("import-error:" + e.constructor.name);
    console.log("import-message:" + (e.message || "").substring(0, 500));
  }
}
main();
`;
		await vm.writeFile("/tmp/try-import.mjs", script);

		let stdout = "";

		const { pid } = vm.spawn("node", ["/tmp/try-import.mjs"], {
			onStdout: (data: Uint8Array) => {
				stdout += new TextDecoder().decode(data);
			},
		});

		// The import succeeds but the CLI's top-level code starts running
		// and never completes (hangs), so we kill after 20s.
		const timeout = setTimeout(() => {
			vm.killProcess(pid);
		}, 20_000);

		const exitCode = await vm.waitProcess(pid);
		clearTimeout(timeout);

		expect(stdout).toContain("attempting-import");
		// The ESM bundle loads successfully after secure-exec fixes
		expect(stdout).toContain("import-success");
		// Exit code 143 = killed by timeout (SIGTERM) because startup hangs
		expect(exitCode).toBe(143);
	}, 30_000);

	test("cli.js --version hangs during startup", async () => {
		// Even with all secure-exec fixes, the CLI cannot complete startup.
		// It no longer crashes (used to fail with "filename must be a string or URL"
		// before the import.meta.url fix), but it hangs during the complex
		// initialization sequence that requires terminal, network, and config access.
		let stdout = "";

		const cliPath = "/root/node_modules/@anthropic-ai/claude-code/cli.js";

		const { pid } = vm.spawn("node", [cliPath, "--version"], {
			onStdout: (data: Uint8Array) => {
				stdout += new TextDecoder().decode(data);
			},
			env: {
				CLAUDE_CODE_DISABLE_TERMINAL_TITLE: "1",
			},
		});

		const timeout = setTimeout(() => {
			vm.killProcess(pid);
		}, 15_000);

		const exitCode = await vm.waitProcess(pid);
		clearTimeout(timeout);

		// CLI hangs — killed by timeout. No stdout output.
		expect(exitCode).toBe(143);
		// No version output produced before hang
		expect(stdout).toBe("");
	}, 30_000);
});
