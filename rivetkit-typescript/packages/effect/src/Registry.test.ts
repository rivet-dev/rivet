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
});
