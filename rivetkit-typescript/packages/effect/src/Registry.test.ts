import { assert, describe, it } from "@effect/vitest";
import { Effect, Layer } from "effect";
import * as Registry from "./Registry";
import * as Action from "./Action";
import * as Actor from "./Actor";

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
