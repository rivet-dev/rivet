import { setup } from "@/mod";
import { counter } from "../../fixtures/driver-test-suite/counter";
import { describeDriverMatrix } from "./shared-matrix";
import { getOrStartSharedEngine, TOKEN } from "./shared-harness";
import { setupDriverTest } from "./shared-utils";
import { describe, expect, test } from "vitest";
import { encodeToEnvoy } from "@rivetkit/engine-envoy-protocol";

const ENVOY_PROTOCOL_VERSION = 2;
const ACTOR_NAME = "counter";

function serverlessHeaders(input: {
	endpoint: string;
	poolName: string;
	namespace?: string;
	token?: string;
}): Headers {
	return new Headers({
		"x-rivet-endpoint": input.endpoint,
		...(input.token ? { "x-rivet-token": input.token } : {}),
		"x-rivet-pool-name": input.poolName,
		"x-rivet-namespace-name": input.namespace ?? "default",
	});
}

function makeStartPayload(actorId: string): Buffer {
	const body = encodeToEnvoy({
		tag: "ToEnvoyCommands",
		val: [
			{
				checkpoint: {
					actorId,
					generation: 0,
					index: 0n,
				},
				inner: {
					tag: "CommandStartActor",
					val: {
						config: {
							name: ACTOR_NAME,
							key: null,
							createTs: BigInt(Date.now()),
							input: null,
						},
						hibernatingRequests: [],
						preloadedKv: null,
						sqliteStartupData: null,
					},
				},
			},
		],
	});
	const payload = Buffer.alloc(body.byteLength + 2);
	payload.writeUInt16LE(ENVOY_PROTOCOL_VERSION, 0);
	Buffer.from(body).copy(payload, 2);
	return payload;
}

async function upsertNormalRunnerConfig(input: {
	endpoint: string;
	poolName: string;
	namespace: string;
}): Promise<void> {
	const datacentersResponse = await fetch(
		`${input.endpoint}/datacenters?namespace=${encodeURIComponent(input.namespace)}`,
		{
			headers: {
				Authorization: `Bearer ${TOKEN}`,
			},
		},
	);
	if (!datacentersResponse.ok) {
		throw new Error(
			`failed to list datacenters: ${datacentersResponse.status} ${await datacentersResponse.text()}`,
		);
	}
	const datacenters = (await datacentersResponse.json()) as {
		datacenters: Array<{ name: string }>;
	};
	const datacenter = datacenters.datacenters[0]?.name;
	if (!datacenter) {
		throw new Error("engine returned no datacenters");
	}

	const response = await fetch(
		`${input.endpoint}/runner-configs/${encodeURIComponent(input.poolName)}?namespace=${encodeURIComponent(input.namespace)}`,
		{
			method: "PUT",
			headers: {
				Authorization: `Bearer ${TOKEN}`,
				"Content-Type": "application/json",
			},
			body: JSON.stringify({
				datacenters: {
					[datacenter]: {
						normal: {},
					},
				},
			}),
		},
	);
	if (!response.ok) {
		throw new Error(
			`failed to upsert runner config: ${response.status} ${await response.text()}`,
		);
	}
}

async function expectText(response: Response): Promise<string> {
	const text = await response.text();
	expect(response.status).toBe(200);
	return text;
}

describeDriverMatrix(
	"Serverless Handler",
	(driverTestConfig) => {
		describe("Serverless Handler Tests", () => {
			test("serves root, health, and metadata routes", async () => {
				const registry = setup({
					use: {
						[ACTOR_NAME]: counter,
					},
					noWelcome: true,
					endpoint: "http://127.0.0.1:6420",
					token: TOKEN,
				});

				const root = await registry.handler(
					new Request("http://runner.test/api/rivet/"),
				);
				expect(await expectText(root)).toContain("RivetKit server");

				const health = await registry.handler(
					new Request("http://runner.test/api/rivet/health"),
				);
				expect(health.status).toBe(200);
				expect(await health.json()).toMatchObject({
					status: "ok",
					runtime: "rivetkit",
				});

				const metadata = await registry.handler(
					new Request("http://runner.test/api/rivet/metadata"),
				);
				expect(metadata.status).toBe(200);
				const body = await metadata.json();
				expect(body).toMatchObject({
					runtime: "rivetkit",
					envoy: { kind: { serverless: {} } },
				});
				expect(body.actorNames[ACTOR_NAME]).toBeDefined();
			});

			test("rejects invalid start headers as structured errors", async () => {
				const registry = setup({
					use: {
						[ACTOR_NAME]: counter,
					},
					noWelcome: true,
					endpoint: "http://127.0.0.1:6420",
					token: TOKEN,
				});

				const response = await registry.handler(
					new Request("http://runner.test/api/rivet/start", {
						method: "POST",
						body: new Uint8Array(),
					}),
				);

				expect(response.status).toBe(400);
				expect(await response.json()).toMatchObject({
					group: "request",
					code: "invalid",
					metadata: {
						reason: "x-rivet-endpoint header is required",
					},
				});
			});

			test("accepts a serverless start payload and streams pings", async (c) => {
				const { client, namespace } = await setupDriverTest(c, driverTestConfig);
				const engine = await getOrStartSharedEngine();
				const poolName = `serverless-${crypto.randomUUID()}`;
				await upsertNormalRunnerConfig({
					endpoint: engine.endpoint,
					poolName,
					namespace,
				});
				const actorId = await client.counter
					.getOrCreate([`serverless-start-${crypto.randomUUID()}`])
					.resolve();
				const registry = setup({
					use: {
						[ACTOR_NAME]: counter,
					},
					noWelcome: true,
					endpoint: engine.endpoint,
					token: TOKEN,
					namespace,
					envoy: { poolName },
				});
				const abort = new AbortController();

				const response = await registry.handler(
					new Request("http://runner.test/api/rivet/start", {
						method: "POST",
						headers: serverlessHeaders({
							endpoint: engine.endpoint,
							poolName,
							namespace,
							token: TOKEN,
						}),
						body: makeStartPayload(actorId),
						signal: abort.signal,
					}),
				);

				expect(response.status).toBe(200);
				expect(response.headers.get("content-type")).toBe("text/event-stream");
				const reader = response.body?.getReader();
				expect(reader).toBeDefined();

				const firstChunk = await Promise.race([
					reader!.read(),
					new Promise<never>((_, reject) =>
						setTimeout(() => reject(new Error("timed out waiting for SSE ping")), 10_000),
					),
				]);
				expect(new TextDecoder().decode(firstChunk.value)).toContain(
					"event: ping",
				);

				abort.abort();
				const closed = await Promise.race([
					reader!.read(),
					new Promise<never>((_, reject) =>
						setTimeout(() => reject(new Error("timed out waiting for SSE close")), 10_000),
					),
				]);
				expect(closed.done).toBe(true);
			});
		});
	},
	{
		registryVariants: ["static"],
		encodings: ["bare"],
	},
);
