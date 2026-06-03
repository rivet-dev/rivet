import { spawn, type ChildProcess } from "node:child_process";
import { randomUUID } from "node:crypto";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { createServer } from "node:net";
import { createClient } from "../rivetkit-typescript/packages/rivetkit/dist/tsup/client/mod.js";
import type { registry } from "../examples/kitchen-sink/src/index.ts";

const TOKEN = "dev";
const HOST = "127.0.0.1";
const WRANGLER_VERSION = "4.86.0";
let lastEngineOutput = "";
let lastWorkerdOutput = "";

function freePort(): Promise<number> {
	return new Promise((resolvePort, reject) => {
		const server = createServer();
		server.once("error", reject);
		server.listen(0, HOST, () => {
			const address = server.address();
			if (!address || typeof address === "string") {
				server.close(() => reject(new Error("failed to allocate port")));
				return;
			}
			const port = address.port;
			server.close(() => resolvePort(port));
		});
	});
}

async function waitForOk(url: string, timeoutMs: number): Promise<void> {
	const deadline = Date.now() + timeoutMs;
	let lastError: unknown;
	while (Date.now() < deadline) {
		try {
			const res = await fetchWithTimeout(url, undefined, 2_000);
			if (res.ok) return;
			lastError = new Error(`${res.status} ${await res.text()}`);
		} catch (error) {
			lastError = error;
		}
		await new Promise((resolve) => setTimeout(resolve, 250));
	}
	throw new Error(`timed out waiting for ${url}: ${String(lastError)}`);
}

async function readJson<T>(res: Response): Promise<T> {
	const text = await res.text();
	if (!res.ok) {
		throw new Error(`${res.status} ${text}`);
	}
	return JSON.parse(text) as T;
}

async function fetchWithTimeout(
	input: string,
	init?: RequestInit,
	timeoutMs = 15_000,
): Promise<Response> {
	const controller = new AbortController();
	const timeout = setTimeout(() => controller.abort(), timeoutMs);
	try {
		return await fetch(input, { ...init, signal: init?.signal ?? controller.signal });
	} finally {
		clearTimeout(timeout);
	}
}

function logStep(step: string, details?: Record<string, unknown>) {
	console.error(JSON.stringify({ kind: "step", step, ...details }));
}

async function waitForWebSocketOpen(ws: WebSocket): Promise<void> {
	if (ws.readyState === WebSocket.OPEN) return;
	await new Promise<void>((resolve, reject) => {
		ws.addEventListener("open", () => resolve(), { once: true });
		ws.addEventListener("error", () => reject(new Error("websocket error")), {
			once: true,
		});
		ws.addEventListener(
			"close",
			(event) =>
				reject(
					new Error(
						`websocket closed before open code=${event.code} reason=${event.reason}`,
					),
				),
			{ once: true },
		);
	});
}

async function nextJsonMessage<T>(ws: WebSocket, timeoutMs = 5_000): Promise<T> {
	return await new Promise<T>((resolve, reject) => {
		const timeout = setTimeout(
			() => reject(new Error("timed out waiting for websocket message")),
			timeoutMs,
		);
		ws.addEventListener(
			"message",
			(event) => {
				clearTimeout(timeout);
				resolve(JSON.parse(String(event.data)) as T);
			},
			{ once: true },
		);
		ws.addEventListener(
			"close",
			(event) => {
				clearTimeout(timeout);
				reject(
					new Error(`websocket closed code=${event.code} reason=${event.reason}`),
				);
			},
			{ once: true },
		);
	});
}

function spawnLogged(
	command: string,
	args: string[],
	options: { env?: NodeJS.ProcessEnv } = {},
) {
	const child = spawn(command, args, {
		env: { ...process.env, ...options.env },
		stdio: ["ignore", "pipe", "pipe"],
	});
	return child;
}

async function stopChild(child: ChildProcess | undefined) {
	if (!child || child.exitCode !== null) return;
	child.kill("SIGTERM");
	await new Promise((resolve) => setTimeout(resolve, 1000));
	if (child.exitCode === null) {
		child.kill("SIGKILL");
	}
}

async function main() {
	const guardPort = await freePort();
	const apiPeerPort = await freePort();
	const metricsPort = await freePort();
	const workerdPort = await freePort();
	const endpoint = `http://${HOST}:${guardPort}`;
	const serviceUrl = `http://${HOST}:${workerdPort}/api/rivet`;
	const namespace = `workerd-e2e-${randomUUID()}`;
	const runnerName = `workerd-kitchen-sink-${randomUUID()}`;
	const dbRoot = mkdtempSync(join(tmpdir(), "rivetkit-workerd-e2e-"));
	const configPath = join(dbRoot, "engine.json");
	const wranglerConfigPath = join(dbRoot, "wrangler.toml");
	let engine: ChildProcess | undefined;
	let workerd: ChildProcess | undefined;

	try {
		writeFileSync(
			configPath,
			JSON.stringify({
				topology: {
					datacenter_label: 1,
					datacenters: {
						default: {
							datacenter_label: 1,
							is_leader: true,
							public_url: endpoint,
							peer_url: `http://${HOST}:${apiPeerPort}`,
						},
					},
				},
			}),
		);

		writeFileSync(
			wranglerConfigPath,
			[
				'name = "rivetkit-kitchen-sink-workerd-e2e"',
				'main = "./repo/examples/kitchen-sink/src/cloudflare.ts"',
				'compatibility_date = "2026-05-01"',
				'compatibility_flags = ["nodejs_compat"]',
				"",
				"[[rules]]",
				'type = "CompiledWasm"',
				'globs = ["**/*.wasm"]',
				"fallthrough = true",
				"",
			].join("\n"),
		);

		const repoLink = join(dbRoot, "repo");
		await import("node:fs/promises").then((fs) =>
			fs.symlink(resolve("."), repoLink, "dir"),
		);

		engine = spawnLogged(resolve("target/debug/rivet-engine"), [
			"--config",
			configPath,
			"start",
		], {
			env: {
				RIVET__GUARD__HOST: HOST,
				RIVET__GUARD__PORT: guardPort.toString(),
				RIVET__API_PEER__HOST: HOST,
				RIVET__API_PEER__PORT: apiPeerPort.toString(),
				RIVET__METRICS__HOST: HOST,
				RIVET__METRICS__PORT: metricsPort.toString(),
				RIVET__FILE_SYSTEM__PATH: join(dbRoot, "db"),
			},
		});
		engine.stdout?.on("data", (chunk) => {
			lastEngineOutput += chunk.toString();
		});
		engine.stderr?.on("data", (chunk) => {
			lastEngineOutput += chunk.toString();
		});

		logStep("wait-engine", { endpoint });
		await waitForOk(`${endpoint}/health`, 90_000);

		workerd = spawnLogged("npx", [
			"-y",
			`wrangler@${WRANGLER_VERSION}`,
			"dev",
			"--config",
			wranglerConfigPath,
			"--ip",
			HOST,
			"--port",
			workerdPort.toString(),
			"--local",
		], {
			env: {
				CI: "1",
				WRANGLER_SEND_METRICS: "false",
				RIVET_LOG_LEVEL: "debug",
			},
		});
		workerd.stdout?.on("data", (chunk) => {
			lastWorkerdOutput += chunk.toString();
		});
		workerd.stderr?.on("data", (chunk) => {
			lastWorkerdOutput += chunk.toString();
		});

		logStep("wait-workerd", { serviceUrl });
		await waitForOk(`http://${HOST}:${workerdPort}/health`, 120_000);

		logStep("metadata");
		const serviceMetadata = await readJson<{ runtime: string }>(
			await fetchWithTimeout(`${serviceUrl}/metadata`),
		);
		if (serviceMetadata.runtime !== "rivetkit") {
			throw new Error(`unexpected metadata runtime ${serviceMetadata.runtime}`);
		}

		logStep("create-namespace", { namespace });
		await readJson(
			await fetchWithTimeout(`${endpoint}/namespaces`, {
				method: "POST",
				headers: {
					Authorization: `Bearer ${TOKEN}`,
					"Content-Type": "application/json",
				},
				body: JSON.stringify({ name: namespace, display_name: namespace }),
			}),
		);

		logStep("get-datacenters", { namespace });
		const datacenters = await readJson<{ datacenters: Array<{ name: string }> }>(
			await fetchWithTimeout(`${endpoint}/datacenters?namespace=${namespace}`, {
				headers: { Authorization: `Bearer ${TOKEN}` },
			}),
		);
		const dc = datacenters.datacenters[0]?.name;
		if (!dc) throw new Error("engine returned no datacenters");

		logStep("serverless-health-check", { serviceUrl });
		const healthCheck = await readJson<{ success?: { version: string }; failure?: unknown }>(
			await fetchWithTimeout(
				`${endpoint}/runner-configs/serverless-health-check?namespace=${namespace}`,
				{
					method: "POST",
					headers: {
						Authorization: `Bearer ${TOKEN}`,
						"Content-Type": "application/json",
					},
					body: JSON.stringify({ url: serviceUrl, headers: {} }),
				},
				30_000,
			),
		);
		if (!("success" in healthCheck)) {
			throw new Error(`serverless health check failed: ${JSON.stringify(healthCheck)}`);
		}

		logStep("put-runner-config", { runnerName, dc });
		await readJson(
			await fetchWithTimeout(
				`${endpoint}/runner-configs/${encodeURIComponent(runnerName)}?namespace=${namespace}`,
				{
					method: "PUT",
					headers: {
						Authorization: `Bearer ${TOKEN}`,
						"Content-Type": "application/json",
					},
					body: JSON.stringify({
						datacenters: {
							[dc]: {
								serverless: {
									url: serviceUrl,
									headers: { "x-rivet-token": TOKEN },
									request_lifespan: 30,
									max_concurrent_actors: 8,
									drain_grace_period: 10,
									slots_per_runner: 8,
									min_runners: 0,
									max_runners: 8,
									runners_margin: 0,
									metadata_poll_interval: 1000,
								},
								drain_on_version_upgrade: true,
							},
						},
					}),
				},
			),
		);

		const client = createClient<typeof registry>({
			endpoint,
			namespace,
			token: TOKEN,
			poolName: runnerName,
			disableMetadataLookup: true,
		});
		try {
			logStep("counter-action");
			const count = await client.counter
				.getOrCreate(["workerd-counter"])
				.increment(7);
			if (count !== 7) {
				throw new Error(`expected counter result 7, received ${count}`);
			}

			logStep("sqlite-action");
			const sqliteActor = client.testCounterSqlite.getOrCreate(["workerd-sqlite"]);
			const sqliteCount = await sqliteActor.increment(11);
			if (sqliteCount !== 11) {
				throw new Error(`expected sqlite count 11, received ${sqliteCount}`);
			}
			const sqliteReadback = await sqliteActor.getCount();
			if (sqliteReadback !== 11) {
				throw new Error(`expected sqlite readback 11, received ${sqliteReadback}`);
			}

			logStep("raw-http");
			const httpResponse = await client.rawHttpActor
				.getOrCreate(["workerd-http"])
				.fetch("api/hello");
			const httpBody = await readJson<{ message: string }>(httpResponse);
			if (httpBody.message !== "Hello from actor!") {
				throw new Error(`unexpected raw HTTP body ${JSON.stringify(httpBody)}`);
			}

			logStep("raw-websocket");
			const ws = await client.rawWebSocketActor
				.getOrCreate(["workerd-websocket"])
				.webSocket();
			try {
				await waitForWebSocketOpen(ws);
				const welcome = await nextJsonMessage<{ type: string }>(ws);
				if (welcome.type !== "welcome") {
					throw new Error(`unexpected websocket welcome ${JSON.stringify(welcome)}`);
				}
				ws.send(JSON.stringify({ type: "ping" }));
				const pong = await nextJsonMessage<{ type: string }>(ws);
				if (pong.type !== "pong") {
					throw new Error(`unexpected websocket pong ${JSON.stringify(pong)}`);
				}
			} finally {
				ws.close();
			}
		} finally {
			await client.dispose();
		}

		console.log(
			JSON.stringify({
				ok: true,
				endpoint,
				namespace,
				runnerName,
				serviceUrl,
			}),
		);

		if (engine.exitCode !== null) {
			throw new Error(`engine exited early:\n${lastEngineOutput}`);
		}
		if (workerd.exitCode !== null) {
			throw new Error(`workerd exited early:\n${lastWorkerdOutput}`);
		}
	} finally {
		await stopChild(workerd);
		await stopChild(engine);
		rmSync(dbRoot, { recursive: true, force: true });
	}
}

main()
	.then(() => process.exit(0))
	.catch((error) => {
		console.error(error);
		console.error("=== workerd output ===");
		console.error(lastWorkerdOutput);
		console.error("=== engine output ===");
		console.error(lastEngineOutput);
		process.exit(1);
	});
