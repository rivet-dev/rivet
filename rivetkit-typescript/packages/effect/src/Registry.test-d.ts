import { Context, Effect, Layer, Scope } from "effect";
import {
	HttpServerError,
	HttpServerRequest,
	HttpServerResponse,
} from "effect/unstable/http";
import { describe, expectTypeOf, test } from "vitest";
import * as Action from "./Action";
import * as Actor from "./Actor";
import * as Registry from "./Registry";

const TestActor = Actor.make("TestActor", {
	actions: [Action.make("Test")],
});

const TestActorLive = TestActor.toLayer({
	Test: () => Effect.void,
});

const RegistryLive = TestActorLive.pipe(
	Layer.provideMerge(Registry.layer({ endpoint: "http://127.0.0.1:6420" })),
);

describe("Registry.layer", () => {
	test("accepts connection options", () => {
		expectTypeOf(Registry.layer).toBeCallableWith({
			endpoint: "http://127.0.0.1:6420",
			token: "dev-token",
			namespace: "default",
		});
	});

	test("does not accept serverless options", () => {
		Registry.layer({
			// @ts-expect-error: serverless routing belongs to toWebHandler and toHttpEffect options.
			serverless: {
				basePath: "/",
			},
		});
	});
});

describe("Registry.toWebHandler", () => {
	test("accepts a registry layer", () => {
		expectTypeOf(Registry.toWebHandler).toBeCallableWith(RegistryLive);
	});

	test("rejects actor registration layers that do not provide Registry", () => {
		// @ts-expect-error: actor registration layers require Registry but do not provide it.
		Registry.toWebHandler(TestActorLive);
	});

	test("accepts serverless routing options", () => {
		expectTypeOf(Registry.toWebHandler).toBeCallableWith(RegistryLive, {
			serverless: {
				basePath: "/",
				maxStartPayloadBytes: 1024,
			},
		});
	});

	test("returns a Fetch-compatible handler", () => {
		const handler = Registry.toWebHandler(RegistryLive);

		expectTypeOf(handler.handler).toEqualTypeOf<
			(
				request: Request,
				context?: Context.Context<never> | undefined,
			) => Promise<Response>
		>();
		expectTypeOf(handler.dispose).toEqualTypeOf<() => Promise<void>>();
	});
});

describe("Registry.toHttpEffect", () => {
	test("accepts serverless routing options", () => {
		expectTypeOf(Registry.toHttpEffect).toBeCallableWith(RegistryLive, {
			serverless: {
				basePath: "/",
				maxStartPayloadBytes: 1024,
			},
		});
	});

	test("rejects actor registration layers that do not provide Registry", () => {
		// @ts-expect-error: actor registration layers require Registry but do not provide it.
		Registry.toHttpEffect(TestActorLive);
	});

	test("returns a scoped Effect HTTP handler", () => {
		expectTypeOf(
			Registry.toHttpEffect(RegistryLive),
		).toEqualTypeOf<
			Effect.Effect<
				Effect.Effect<
					HttpServerResponse.HttpServerResponse,
					HttpServerError.HttpServerError,
					HttpServerRequest.HttpServerRequest
				>,
				never,
				Scope.Scope
			>
		>();
	});
});
