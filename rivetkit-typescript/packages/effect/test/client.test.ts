import { Buffer } from "node:buffer";
import { once } from "node:events";
import { createServer, type IncomingHttpHeaders, type Server } from "node:http";
import { Action, Actor, Client } from "@rivetkit/effect";
import { Effect, Schema } from "effect";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

class Rejected extends Schema.TaggedError<Rejected>()("Rejected", {
	message: Schema.String,
}) {}

const Echo = Action.make("Echo", {
	payload: { value: Schema.String },
	success: Schema.String,
	error: Rejected,
});

const TestActor = Actor.make("testActor", { actions: [Echo] });

describe("client options", () => {
	let server: Server;
	let endpoint: string;
	let requests: Array<{ url: URL; headers: IncomingHttpHeaders; body: unknown }>;
	let response: { status: number; body: unknown };

	beforeEach(async () => {
		requests = [];
		response = { status: 200, body: { output: "hello" } };
		server = createServer(async (req, res) => {
			const chunks: Array<Buffer> = [];
			for await (const chunk of req) {
				chunks.push(Buffer.from(chunk));
			}
			requests.push({
				url: new URL(req.url!, endpoint),
				headers: req.headers,
				body: JSON.parse(Buffer.concat(chunks).toString()),
			});
			res.writeHead(response.status, { "content-type": "application/json" });
			res.end(JSON.stringify(response.body));
		});
		server.listen(0, "127.0.0.1");
		await once(server, "listening");
		const address = server.address();
		if (address === null || typeof address === "string") {
			throw new Error("Expected an HTTP server address");
		}
		endpoint = `http://127.0.0.1:${address.port}`;
	});

	afterEach(async () => {
		await new Promise<void>((resolve, reject) =>
			server.close((error) => (error ? reject(error) : resolve())),
		);
	});

	const options = (): Client.Options => ({
		endpoint,
		token: "test-token",
		namespace: "test-namespace",
		poolName: "test-pool",
		encoding: "json",
		headers: { "x-test-header": "forwarded" },
		disableMetadataLookup: true,
		devtools: false,
	});

	it("forwards connection params, region, input, and client configuration", async () => {
		const result = await Effect.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const client = yield* Client.make(options());
					const actor = client.makeActorAccessor(TestActor).getOrCreate("key", {
						params: { token: "actor-token" },
						createInRegion: "local",
						createWithInput: { initial: 1 },
					});
					return yield* actor.Echo({ value: "hello" });
				}),
			),
		);
		expect(result).toBe("hello");
		expect(requests).toHaveLength(1);
		const { url, headers, body } = requests[0];
		expect(url.pathname).toBe("/gateway/testActor/action/Echo");
		expect(url.searchParams.get("rvt-namespace")).toBe("test-namespace");
		expect(url.searchParams.get("rvt-runner")).toBe("test-pool");
		expect(url.searchParams.get("rvt-key")).toBe("key");
		expect(url.searchParams.get("rvt-region")).toBe("local");
		expect(url.searchParams.get("rvt-input")).toBeTruthy();
		expect(headers["x-rivet-conn-params"]).toBe('{"token":"actor-token"}');
		expect(headers["x-test-header"]).toBe("forwarded");
		expect(headers["x-rivet-token"]).toBe("test-token");
		expect(body).toMatchObject({ args: [{ value: "hello" }, { trace: {} }] });
	});

	it.each([{ key: "key" }, { key: ["tenant", "key"] }])(
		"supports key-only calls with key $key",
		async ({ key }) => {
			await Effect.runPromise(
				Effect.gen(function* () {
					const client = yield* TestActor.client;
					return yield* client.getOrCreate(key).Echo({ value: "hello" });
				}).pipe(Effect.provide(Client.layer(options()))),
			);
			expect(requests).toHaveLength(1);
			expect(requests[0].url.searchParams.get("rvt-key")).toBe(
				Array.isArray(key) ? key.join(",") : key,
			);
			expect(requests[0].headers["x-rivet-conn-params"]).toBeUndefined();
			expect(requests[0].url.searchParams.has("rvt-region")).toBe(false);
		},
	);

	it("keeps per-call options isolated between handles", async () => {
		await Effect.runPromise(
			Effect.gen(function* () {
				const client = yield* TestActor.client;
				const configured = client.getOrCreate("key", {
					params: { token: "actor-token" },
					createInRegion: "local",
					poolName: "override-pool",
				});
				const plain = client.getOrCreate("key");
				yield* configured.Echo({ value: "hello" });
				yield* plain.Echo({ value: "hello" });
			}).pipe(Effect.provide(Client.layer(options()))),
		);
		expect(requests).toHaveLength(2);
		expect(requests[0].headers["x-rivet-conn-params"]).toBe(
			'{"token":"actor-token"}',
		);
		expect(requests[0].url.searchParams.get("rvt-region")).toBe("local");
		expect(requests[0].url.searchParams.get("rvt-runner")).toBe("override-pool");
		expect(requests[1].headers["x-rivet-conn-params"]).toBeUndefined();
		expect(requests[1].url.searchParams.has("rvt-region")).toBe(false);
		expect(requests[1].url.searchParams.get("rvt-runner")).toBe("test-pool");
	});

	it("preserves typed action errors with per-call options and a pool override", async () => {
		response = {
			status: 500,
			body: {
				group: "action",
				code: "rejected",
				message: "Rejected",
				metadata: {
					_tag: "EffectActionError",
					version: 1,
					error: { _tag: "Rejected", message: "Access denied" },
				},
			},
		};
		const error = await Effect.runPromise(
			Effect.gen(function* () {
				const client = yield* TestActor.client;
				return yield* client
					.getOrCreate("key", {
						params: { token: "actor-token" },
						createInRegion: "local",
						poolName: "override-pool",
					})
					.Echo({ value: "hello" })
					.pipe(Effect.flip);
			}).pipe(Effect.provide(Client.layer(options()))),
		);
		expect(error).toBeInstanceOf(Rejected);
		expect(error.message).toBe("Access denied");
		expect(requests).toHaveLength(1);
		expect(requests[0].url.searchParams.get("rvt-runner")).toBe("override-pool");
	});
});
