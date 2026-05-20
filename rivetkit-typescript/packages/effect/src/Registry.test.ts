import { assert, describe, it } from "@effect/vitest";
import { Effect, Layer } from "effect";
import { HttpEffect } from "effect/unstable/http";
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
			serverless: {
				basePath: "/",
			},
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
		const { handler, dispose } =
			Registry.toWebHandler(FinalizedRegistryLive);

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

				assert.strictEqual(response.status, 200);
				const body = (yield* Effect.promise(() => response.json())) as {
					readonly actorNames: Record<string, unknown>;
				};
				assert.ok(body.actorNames.TestActor);
			}),
		),
	);

	it.effect("uses a custom serverless base path", () =>
		Effect.scoped(
			Effect.gen(function* () {
				const httpEffect = yield* Registry.toHttpEffect(RegistryLive, {
					serverless: {
						basePath: "/",
					},
				});
				const handler = HttpEffect.toWebHandler(httpEffect);
				const response = yield* Effect.promise(() =>
					handler(new Request("http://runner.test/metadata")),
				);

				assert.strictEqual(response.status, 200);
				const body = (yield* Effect.promise(() => response.json())) as {
					readonly actorNames: Record<string, unknown>;
				};
				assert.ok(body.actorNames.TestActor);
			}),
		),
	);
});
