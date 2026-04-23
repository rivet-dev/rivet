import { spawn, type ChildProcess } from "node:child_process";
import { randomUUID } from "node:crypto";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { createServer } from "node:net";
import { Hono } from "hono";
import { serve } from "@hono/node-server";
import { createClient } from "rivetkit/client";
import { registry } from "../examples/kitchen-sink/src/index.ts";

const TOKEN = "dev";
const HOST = "127.0.0.1";
let lastEngineOutput = "";

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
			const res = await fetch(url);
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

async function main() {
	const guardPort = await freePort();
	const apiPeerPort = await freePort();
	const metricsPort = await freePort();
	const servicePort = await freePort();
	const endpoint = `http://${HOST}:${guardPort}`;
	const serviceUrl = `http://${HOST}:${servicePort}/api/rivet`;
	const namespace = `serverless-e2e-${randomUUID()}`;
	const runnerName = `kitchen-sink-${randomUUID()}`;
	const dbRoot = mkdtempSync(join(tmpdir(), "rivetkit-serverless-e2e-"));
	const configPath = join(dbRoot, "engine.json");
	let engine: ChildProcess | undefined;
	let service: ReturnType<typeof serve> | undefined;

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

		engine = spawn(resolve("target/debug/rivet-engine"), ["--config", configPath, "start"], {
			env: {
				...process.env,
				RIVET__GUARD__HOST: HOST,
				RIVET__GUARD__PORT: guardPort.toString(),
				RIVET__API_PEER__HOST: HOST,
				RIVET__API_PEER__PORT: apiPeerPort.toString(),
				RIVET__METRICS__HOST: HOST,
				RIVET__METRICS__PORT: metricsPort.toString(),
				RIVET__FILE_SYSTEM__PATH: join(dbRoot, "db"),
			},
			stdio: ["ignore", "pipe", "pipe"],
		});

		engine.stdout?.on("data", (chunk) => {
			lastEngineOutput += chunk.toString();
		});
		engine.stderr?.on("data", (chunk) => {
			lastEngineOutput += chunk.toString();
		});

		logStep("wait-engine", { endpoint });
		await waitForOk(`${endpoint}/health`, 90_000);

		registry.config.test = { ...registry.config.test, enabled: true };
		registry.config.startEngine = false;
		registry.config.endpoint = endpoint;
		registry.config.token = TOKEN;
		registry.config.namespace = namespace;
		registry.config.envoy = {
			...registry.config.envoy,
			poolName: runnerName,
		};

		const app = new Hono();
		app.all("/api/rivet/*", async (c) => {
			const res = await registry.handler(c.req.raw);
			console.error(
				JSON.stringify({
					kind: "serverless-request",
					method: c.req.method,
					path: new URL(c.req.url).pathname,
					status: res.status,
					endpoint: c.req.header("x-rivet-endpoint"),
					poolName: c.req.header("x-rivet-pool-name"),
					namespace: c.req.header("x-rivet-namespace-name"),
					hasToken: Boolean(c.req.header("x-rivet-token")),
				}),
			);
			return res;
		});
		app.get("/health", (c) => c.json({ ok: true }));
		service = serve({ fetch: app.fetch, hostname: HOST, port: servicePort });
		logStep("wait-service", { serviceUrl });
		await waitForOk(`http://${HOST}:${servicePort}/health`, 10_000);

		logStep("metadata");
		const serviceMetadata = await readJson<{ runtime: string; actorNames: unknown }>(
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
				body: JSON.stringify({
					name: namespace,
					display_name: namespace,
				}),
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
			logStep("actor-increment");
			const handle = client.counter.getOrCreate(["serverless-e2e"]);
			const count = await Promise.race([
				handle.increment(7),
				new Promise<never>((_, reject) =>
					setTimeout(() => reject(new Error("actor increment timed out")), 60_000),
				),
			]);
			if (count !== 7) {
				throw new Error(`expected counter result 7, received ${count}`);
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
	} finally {
		service?.close();
		if (engine && engine.exitCode === null) {
			engine.kill("SIGTERM");
			await new Promise((resolve) => setTimeout(resolve, 1000));
			if (engine.exitCode === null) engine.kill("SIGKILL");
		}
		rmSync(dbRoot, { recursive: true, force: true });
	}
}

main()
	.then(() => process.exit(0))
	.catch((error) => {
		console.error(error);
		console.error(lastEngineOutput);
		process.exit(1);
	});
