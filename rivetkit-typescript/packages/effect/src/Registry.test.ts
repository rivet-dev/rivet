import { assert, describe, it } from "@effect/vitest";
import { Effect, Layer } from "effect";
import { HttpEffect } from "effect/unstable/http";
import { vi } from "vitest";
import * as Action from "./Action";
import * as Actor from "./Actor";
import * as Registry from "./Registry";

const TestActor = Actor.make("TestActor", {
	actions: [Action.make("Test")],
});

const TestActorLive = TestActor.toLayer({
	Test: () => Effect.void,
});

const ActorsLayer = Layer.mergeAll(TestActorLive);

const RegistryLive = ActorsLayer.pipe(
	Layer.provideMerge(
		Registry.layer({
			endpoint: "http://127.0.0.1:6420",
			noWelcome: true,
		}),
	),
);

describe("Registry.toWebHandler", () => {
	it("serves registered actors as a Fetch handler", async () => {
		const { handler, dispose } = Registry.toWebHandler(RegistryLive);

		try {
			const response = await handler(
				new Request("http://runner.test/api/rivet/metadata"),
			);

			assert.strictEqual(response.status, 200);
			const body = (await response.json()) as {
				readonly actorNames: Record<string, unknown>;
			};
			await assert.ok(body.actorNames.TestActor);
		} finally {
			await dispose();
		}
	});

	it("uses a custom serverless base path", async () => {
		const { handler, dispose } = Registry.toWebHandler(RegistryLive, {
			basePath: "/",
		});

		try {
			const response = await handler(
				new Request("http://runner.test/metadata"),
			);

			assert.strictEqual(response.status, 200);
			const body = (await response.json()) as {
				readonly actorNames: Record<string, unknown>;
			};
			await assert.ok(body.actorNames.TestActor);
		} finally {
			await dispose();
		}
	});

	it("uses the custom base path to identify start requests", async () => {
		const { handler, dispose } = Registry.toWebHandler(RegistryLive, {
			basePath: "/custom",
			maxStartPayloadBytes: 1,
		});

		try {
			const defaultPrefix = await handler(
				new Request("http://runner.test/api/rivet/start", {
					method: "POST",
					body: new Uint8Array([1, 2]),
				}),
			);
			assert.notStrictEqual(defaultPrefix.status, 413);

			const customPrefix = await handler(
				new Request("http://runner.test/custom/start", {
					method: "POST",
					body: new Uint8Array([1, 2]),
				}),
			);
			assert.strictEqual(customPrefix.status, 413);
			const body = (await customPrefix.json()) as {
				readonly group: string;
				readonly code: string;
				readonly message: string;
			};
			assert.deepStrictEqual(
				{ group: body.group, code: body.code },
				{ group: "message", code: "incoming_too_long" },
			);
			await assert.match(body.message, /limit is 1 bytes/);
		} finally {
			await dispose();
		}
	});

	it("uses a custom serverless start payload size limit", async () => {
		const { handler, dispose } = Registry.toWebHandler(RegistryLive, {
			maxStartPayloadBytes: 1,
		});

		try {
			const response = await handler(
				new Request("http://runner.test/api/rivet/start", {
					method: "POST",
					body: new Uint8Array([1, 2]),
				}),
			);

			assert.strictEqual(response.status, 413);
			const body = (await response.json()) as {
				readonly group: string;
				readonly code: string;
				readonly message: string;
			};
			assert.deepStrictEqual(
				{ group: body.group, code: body.code },
				{ group: "message", code: "incoming_too_long" },
			);
			await assert.match(body.message, /limit is 1 bytes/);
		} finally {
			await dispose();
		}
	});

	it("does not print the welcome banner when disabled", async () => {
		const log = vi.spyOn(console, "log").mockImplementation(() => {});
		const { handler, dispose } = Registry.toWebHandler(RegistryLive);

		try {
			const response = await handler(
				new Request("http://runner.test/api/rivet/metadata"),
			);

			assert.strictEqual(response.status, 200);
			assert.strictEqual(log.mock.calls.length, 0);
		} finally {
			await dispose();
			log.mockRestore();
		}
	});

	it("builds the registry layer once across requests", async () => {
		let builds = 0;
		const CountingRegistryLive = Layer.mergeAll(
			RegistryLive,
			Layer.effectDiscard(
				Effect.sync(() => {
					builds += 1;
				}),
			),
		);
		const { handler, dispose } =
			Registry.toWebHandler(CountingRegistryLive);

		try {
			const first = await handler(
				new Request("http://runner.test/api/rivet/metadata"),
			);
			const second = await handler(
				new Request("http://runner.test/api/rivet/metadata"),
			);

			assert.strictEqual(first.status, 200);
			assert.strictEqual(second.status, 200);
			assert.strictEqual(builds, 1);
		} finally {
			await dispose();
		}
	});

	it("closes registry layer finalizers on dispose", async () => {
		let finalizers = 0;
		const FinalizedRegistryLive = Layer.mergeAll(
			RegistryLive,
			Layer.effectDiscard(
				Effect.addFinalizer(() =>
					Effect.sync(() => {
						finalizers += 1;
					}),
				),
			),
		);
		const { handler, dispose } = Registry.toWebHandler(
			FinalizedRegistryLive,
		);

		try {
			const response = await handler(
				new Request("http://runner.test/api/rivet/metadata"),
			);

			assert.strictEqual(response.status, 200);
			assert.strictEqual(finalizers, 0);
		} finally {
			await dispose();
		}
		assert.strictEqual(finalizers, 1);
	});
});

describe("Registry.toHttpEffect", () => {
	it.effect("serves registered actors as an Effect HTTP handler", () =>
		Effect.scoped(
			Effect.gen(function* () {
				const httpEffect = yield* Registry.toHttpEffect(RegistryLive);
				const handler = HttpEffect.toWebHandler(httpEffect);
				const response = yield* Effect.promise(() =>
					handler(
						new Request("http://runner.test/api/rivet/metadata"),
					),
				);

				yield* Effect.promise(() =>
					(async (response: Response) => {
						assert.strictEqual(response.status, 200);
						const body = (await response.json()) as {
							readonly actorNames: Record<string, unknown>;
						};
						await assert.ok(body.actorNames.TestActor);
					})(response),
				);
			}),
		),
	);

	it.effect("uses a custom serverless base path", () =>
		Effect.scoped(
			Effect.gen(function* () {
				const httpEffect = yield* Registry.toHttpEffect(RegistryLive, {
					basePath: "/",
				});
				const handler = HttpEffect.toWebHandler(httpEffect);
				const response = yield* Effect.promise(() =>
					handler(new Request("http://runner.test/metadata")),
				);

				yield* Effect.promise(() =>
					(async (response: Response) => {
						assert.strictEqual(response.status, 200);
						const body = (await response.json()) as {
							readonly actorNames: Record<string, unknown>;
						};
						await assert.ok(body.actorNames.TestActor);
					})(response),
				);
			}),
		),
	);

	it.effect("uses the custom base path to identify start requests", () =>
		Effect.scoped(
			Effect.gen(function* () {
				const httpEffect = yield* Registry.toHttpEffect(RegistryLive, {
					basePath: "/custom",
					maxStartPayloadBytes: 1,
				});
				const handler = HttpEffect.toWebHandler(httpEffect);
				const defaultPrefix = yield* Effect.promise(() =>
					handler(
						new Request("http://runner.test/api/rivet/start", {
							method: "POST",
							body: new Uint8Array([1, 2]),
						}),
					),
				);
				assert.notStrictEqual(defaultPrefix.status, 413);

				const customPrefix = yield* Effect.promise(() =>
					handler(
						new Request("http://runner.test/custom/start", {
							method: "POST",
							body: new Uint8Array([1, 2]),
						}),
					),
				);
				yield* Effect.promise(() =>
					(async (response: Response) => {
						assert.strictEqual(response.status, 413);
						const body = (await response.json()) as {
							readonly group: string;
							readonly code: string;
							readonly message: string;
						};
						assert.deepStrictEqual(
							{ group: body.group, code: body.code },
							{ group: "message", code: "incoming_too_long" },
						);
						assert.match(body.message, /limit is 1 bytes/);
					})(customPrefix),
				);
			}),
		),
	);

	it.effect("uses a custom serverless start payload size limit", () =>
		Effect.scoped(
			Effect.gen(function* () {
				const httpEffect = yield* Registry.toHttpEffect(RegistryLive, {
					maxStartPayloadBytes: 1,
				});
				const handler = HttpEffect.toWebHandler(httpEffect);
				const response = yield* Effect.promise(() =>
					handler(
						new Request("http://runner.test/api/rivet/start", {
							method: "POST",
							body: new Uint8Array([1, 2]),
						}),
					),
				);

				yield* Effect.promise(() =>
					(async (response: Response) => {
						assert.strictEqual(response.status, 413);
						const body = (await response.json()) as {
							readonly group: string;
							readonly code: string;
							readonly message: string;
						};
						assert.deepStrictEqual(
							{ group: body.group, code: body.code },
							{ group: "message", code: "incoming_too_long" },
						);
						assert.match(body.message, /limit is 1 bytes/);
					})(response),
				);
			}),
		),
	);
});
