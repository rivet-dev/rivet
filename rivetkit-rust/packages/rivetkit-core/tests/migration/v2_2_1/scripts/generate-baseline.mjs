#!/usr/bin/env node
// Phase 1 verifies the published npm library against the current engine:
//
// cargo build -p rivet-engine
// rivetkit-rust/packages/rivetkit-core/tests/migration/v2_2_1/scripts/generate-baseline.mjs --engine-label current-worktree
// cargo test -p rivetkit-core --features sqlite --test integration actor_v2_2_1_snapshot_starts_in_current_rivetkit_core -- --nocapture
//
// Phase 2 verifies the published npm library against the v2.2.1 engine:
//
// git worktree add --detach /tmp/rivet-v2.2.1-engine v2.2.1
// cd /tmp/rivet-v2.2.1-engine
// cargo build -p rivet-engine --target-dir /tmp/rivet-v2.2.1-engine-target
// cd /path/to/current/rivet
// rivetkit-rust/packages/rivetkit-core/tests/migration/v2_2_1/scripts/generate-baseline.mjs --engine-binary /tmp/rivet-v2.2.1-engine-target/debug/rivet-engine --engine-label v2.2.1
// cargo test -p rivetkit-core --features sqlite --test integration actor_v2_2_1_snapshot_starts_in_current_rivetkit_core -- --nocapture
import { spawn, spawnSync } from "node:child_process";
import { createServer } from "node:net";
import {
	cpSync,
	existsSync,
	mkdirSync,
	mkdtempSync,
	rmSync,
	writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const migrationDir = resolve(scriptDir, "..");
const workspaceRoot = resolve(scriptDir, "../../../../../../..");
const args = parseArgs(process.argv.slice(2));
const rivetkitVersion = args.rivetkitVersion ?? "2.2.1";
const outputDir = resolve(args.output ?? resolve(migrationDir, "snapshot"));
const engineBinary = resolve(
	args.engineBinary ?? resolve(workspaceRoot, "target/debug/rivet-engine"),
);

if (!existsSync(engineBinary)) {
	throw new Error(
		`engine binary not found at ${engineBinary}. Run cargo build -p rivet-engine or pass --engine-binary.`,
	);
}
const engineVersion = run(engineBinary, ["--version"], { cwd: workspaceRoot });

const tempDir = mkdtempSync(resolve(tmpdir(), "rivetkit-2-2-1-snapshot-"));
const projectDir = resolve(tempDir, "project");
const dbDir = resolve(tempDir, "engine-db");
mkdirSync(projectDir);
mkdirSync(dbDir);

let engine;
try {
	writeFileSync(
		resolve(projectDir, "package.json"),
		JSON.stringify(
			{
				private: true,
				type: "module",
				dependencies: {
					rivetkit: rivetkitVersion,
					tsx: "4.19.4",
					ws: "^8.18.0",
				},
			},
			null,
			"\t",
		),
	);
	cpSync(
		resolve(scriptDir, "baseline.ts"),
		resolve(projectDir, "main.ts"),
	);

	run("npm", ["install", "--silent"], { cwd: projectDir });

	const guardPort = await pickPort();
	const apiPeerPort = await pickPort();
	const metricsPort = await pickPort();
	const endpoint = `http://127.0.0.1:${guardPort}`;

	engine = spawn(engineBinary, ["start"], {
		cwd: workspaceRoot,
		env: {
			...process.env,
			RIVET__GUARD__HOST: "127.0.0.1",
			RIVET__GUARD__PORT: String(guardPort),
			RIVET__API_PEER__HOST: "127.0.0.1",
			RIVET__API_PEER__PORT: String(apiPeerPort),
			RIVET__METRICS__HOST: "127.0.0.1",
			RIVET__METRICS__PORT: String(metricsPort),
			RIVET__FILE_SYSTEM__PATH: dbDir,
		},
		stdio: ["ignore", "pipe", "pipe"],
	});
	const engineLogs = collectLogs(engine);
	await waitForHealth(endpoint, engine, engineLogs);

	run("npx", ["tsx", "main.ts"], {
		cwd: projectDir,
		env: {
			...process.env,
			RIVET_ENDPOINT: endpoint,
			RIVET_TOKEN: "dev",
			RIVET_NAMESPACE: "default",
		},
	});

	await stopChild(engine);
	engine = undefined;

	rmSync(outputDir, { force: true, recursive: true });
	mkdirSync(outputDir, { recursive: true });
	cpSync(dbDir, resolve(outputDir, "replica-1"), { recursive: true });
	writeFileSync(
		resolve(outputDir, "metadata.json"),
		`${JSON.stringify(
			{
				commit: git(["rev-parse", "--short", "HEAD"]),
				branch: git(["rev-parse", "--abbrev-ref", "HEAD"]),
				generated_at: String(Math.floor(Date.now() / 1000)),
				method: "published-rivetkit-npm",
				rivetkit_version: rivetkitVersion,
				engine_binary: engineBinary,
				engine_version: engineVersion,
				engine_label: args.engineLabel ?? "current-worktree",
				fixture: "tests/migration/v2_2_1/scripts/baseline.ts",
			},
			null,
			"\t",
		)}\n`,
	);

	console.log(`snapshot: ${outputDir}`);
} finally {
	if (engine) {
		await stopChild(engine);
	}
	if (!args.keepTemp) {
		rmSync(tempDir, { force: true, recursive: true });
	} else {
		console.log(`temp: ${tempDir}`);
	}
}

function parseArgs(argv) {
	const out = {};
	for (let i = 0; i < argv.length; i++) {
		const arg = argv[i];
		if (arg === "--keep-temp") {
			out.keepTemp = true;
		} else if (arg === "--rivetkit-version") {
			out.rivetkitVersion = argv[++i];
		} else if (arg === "--engine-binary") {
			out.engineBinary = argv[++i];
		} else if (arg === "--engine-label") {
			out.engineLabel = argv[++i];
		} else if (arg === "--output") {
			out.output = argv[++i];
		} else {
			throw new Error(`unknown argument: ${arg}`);
		}
	}
	return out;
}

function run(command, args, options) {
	const result = spawnSyncLogged(command, args, options);
	if (result.status !== 0) {
		throw new Error(
			`${command} ${args.join(" ")} failed with ${result.status}\n${result.output}`,
		);
	}
	return result.stdout.trim();
}

function spawnSyncLogged(command, args, options) {
	const result = spawnSync(command, args, {
		...options,
		encoding: "utf8",
		env: options?.env ?? process.env,
	});
	return {
		status: result.status,
		stdout: result.stdout ?? "",
		output: `${result.stdout ?? ""}${result.stderr ?? ""}`,
	};
}

function git(args) {
	return run("git", args, { cwd: workspaceRoot });
}

function collectLogs(child) {
	const logs = { stdout: "", stderr: "" };
	child.stdout?.on("data", (chunk) => {
		logs.stdout += chunk.toString();
	});
	child.stderr?.on("data", (chunk) => {
		logs.stderr += chunk.toString();
	});
	return logs;
}

async function waitForHealth(endpoint, child, logs) {
	const deadline = Date.now() + 60_000;
	while (Date.now() < deadline) {
		if (child.exitCode !== null) {
			throw new Error(
				`engine exited before health check passed\n${tail(logs.stdout)}\n${tail(logs.stderr)}`,
			);
		}
		try {
			const response = await fetch(`${endpoint}/health`);
			if (response.ok) {
				return;
			}
		} catch {}
		await new Promise((resolve) => setTimeout(resolve, 250));
	}
	throw new Error(
		`timed out waiting for engine health\n${tail(logs.stdout)}\n${tail(logs.stderr)}`,
	);
}

async function stopChild(child) {
	if (child.exitCode !== null) {
		return;
	}
	child.kill("SIGTERM");
	const exited = await Promise.race([
		new Promise((resolve) => child.once("exit", resolve)),
		new Promise((resolve) => setTimeout(() => resolve(false), 5000)),
	]);
	if (exited === false && child.exitCode === null) {
		child.kill("SIGKILL");
		await new Promise((resolve) => child.once("exit", resolve));
	}
}

async function pickPort() {
	return await new Promise((resolve, reject) => {
		const server = createServer();
		server.listen(0, "127.0.0.1", () => {
			const address = server.address();
			if (!address || typeof address === "string") {
				reject(new Error("failed to pick port"));
				return;
			}
			const port = address.port;
			server.close(() => resolve(port));
		});
		server.on("error", reject);
	});
}

function tail(text) {
	return text.split("\n").slice(-120).join("\n");
}
