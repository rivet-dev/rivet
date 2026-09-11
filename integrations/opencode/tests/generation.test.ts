import { createServer } from "node:http";
import { describe, expect, it, vi } from "vitest";
import { opencode } from "../dist/index.js";
import { sqliteFixture } from "./sqlite-fixture.js";

describe("embedded generation", () => {
	it("keeps accepted prompts awake and persists generated messages and events", async () => {
		let release!: () => void;
		let started!: () => void;
		let responseGate = new Promise<void>((resolve) => {
			release = resolve;
		});
		let requestStarted = new Promise<void>((resolve) => {
			started = resolve;
		});
		const server = createServer(async (request, response) => {
			request.resume();
			started();
			await responseGate;
			response.writeHead(200, { "Content-Type": "text/event-stream" });
			for (const chunk of [
				{
					choices: [
						{
							index: 0,
							delta: { role: "assistant", content: "Deterministic answer" },
							finish_reason: null,
						},
					],
				},
				{
					choices: [{ index: 0, delta: {}, finish_reason: "stop" }],
					usage: { prompt_tokens: 1, completion_tokens: 1, total_tokens: 2 },
				},
			])
				response.write(
					`data: ${JSON.stringify({ id: "chatcmpl-test", object: "chat.completion.chunk", created: 1, model: "mock", ...chunk })}\n\n`,
				);
			response.end("data: [DONE]\n\n");
		});
		await new Promise<void>((resolve) =>
			server.listen(0, "127.0.0.1", resolve),
		);
		const port = (server.address() as { port: number }).port;
		const { db } = sqliteFixture();
		const definition = opencode({
			opencode: {
				models: { fetch: false },
				config: {
					project: false,
					content: JSON.stringify({
						model: "test/mock",
						snapshots: false,
						providers: {
							test: {
								package: "aisdk:@ai-sdk/openai-compatible",
								settings: {
									apiKey: "unused",
									baseURL: `http://127.0.0.1:${port}/v1`,
								},
								models: {
									mock: {
										name: "mock",
										limit: { context: 32000, output: 1000 },
									},
								},
							},
						},
					}),
				},
			},
			session: { model: { providerID: "test", id: "mock" } },
		});
		let awake = 0;
		const context: any = {
			actorId: "generation-test",
			key: [],
			db,
			client: () => ({}),
			keepAwake: (promise: Promise<unknown>) => {
				awake++;
				return promise.finally(() => {
					awake--;
				});
			},
			broadcast: vi.fn(),
			log: { error: vi.fn() },
		};
		const config = definition.config as any;
		await db.execute(
			"CREATE TABLE _rivet_opencode (id INTEGER PRIMARY KEY, binding TEXT, cwd TEXT NOT NULL)",
		);
		try {
			await config.onWake(context);
			await config.actions.prompt(context, "Say hello");
			await requestStarted;
			expect(awake).toBeGreaterThan(0);
			release();
			await config.actions.waitForIdle(context);
			const messages = await config.actions.getMessages(context);
			expect(messages.data).toContainEqual(
				expect.objectContaining({
					type: "assistant",
					finish: "stop",
					content: [{ type: "text", text: "Deterministic answer" }],
				}),
			);
			const session = await config.actions.getSession(context);
			expect(
				(await config.actions.readEvents(context, { sessionID: session.id }))
					.length,
			).toBeGreaterThan(0);
			await config.onSleep(context);
			await config.onWake(context);
			expect(await config.actions.getMessages(context)).toEqual(messages);

			// Force teardown with a generation in flight, like host eviction. The
			// next wake must resume OpenCode's durable execution claim.
			responseGate = new Promise<void>((resolve) => {
				release = resolve;
			});
			requestStarted = new Promise<void>((resolve) => {
				started = resolve;
			});
			await config.actions.prompt(context, "Resume this turn after eviction");
			await requestStarted;
			await config.onSleep(context);
			release();
			await config.onWake(context);
			await config.actions.waitForIdle(context);
			const recovered = await config.actions.getMessages(context);
			expect(
				recovered.data.filter(
					(message: { type: string; finish?: string }) =>
						message.type === "assistant" && message.finish === "stop",
				),
			).toHaveLength(2);
		} finally {
			release();
			await config.onDestroy(context);
			await db.close();
			server.closeAllConnections();
			await new Promise<void>((resolve) => server.close(() => resolve()));
		}
	}, 30_000);
});
