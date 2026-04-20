import { describeDriverMatrix } from "./shared-matrix";
import { describe, expect, test } from "vitest";
import { setupDriverTest } from "./shared-utils";

describeDriverMatrix("Lifecycle Hooks", (driverTestConfig) => {
	describe("Lifecycle Hooks", () => {
		describe("onBeforeConnect", () => {
			test("rejects connection with UserError", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const conn = client.beforeConnectRejectActor
					.getOrCreate()
					.connect({ shouldReject: true });

				await expect(conn.ping()).rejects.toThrow();

				await conn.dispose();
			});

			test("allows connection when onBeforeConnect succeeds", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const conn = client.beforeConnectRejectActor
					.getOrCreate()
					.connect({ shouldReject: false });

				const result = await conn.ping();
				expect(result).toBe("pong");

				await conn.dispose();
			});

			test("rejects connection with generic error", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const conn = client.beforeConnectGenericErrorActor
					.getOrCreate()
					.connect({ shouldFail: true });

				await expect(conn.ping()).rejects.toThrow();

				await conn.dispose();
			});

			test("allows connection when generic error actor succeeds", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const conn = client.beforeConnectGenericErrorActor
					.getOrCreate()
					.connect({ shouldFail: false });

				const result = await conn.ping();
				expect(result).toBe("pong");

				await conn.dispose();
			});
		});

		describe("onStateChange recursion prevention", () => {
			test("vars writes in onStateChange do not trigger recursive calls", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const actor = client.stateChangeRecursionActor.getOrCreate();

				await actor.setValue(5);

				const all = await actor.getAll();

				// onStateChange should have been called exactly once for the setValue call
				expect(all.callCount).toBe(1);

				expect(all.derivedValue).toBe(10);
			});

			test("multiple state changes each trigger one onStateChange", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const actor = client.stateChangeRecursionActor.getOrCreate();

				await actor.setValue(1);
				await actor.setValue(2);
				await actor.setValue(3);

				const all = await actor.getAll();

				// Three setValue calls, each triggers exactly one onStateChange
				expect(all.callCount).toBe(3);
				expect(all.value).toBe(3);
				expect(all.derivedValue).toBe(6);
			});

			test("reading state does not trigger onStateChange", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const actor = client.stateChangeRecursionActor.getOrCreate();

				await actor.setValue(10);

				// Read-only operations
				await actor.getDerivedValue();
				await actor.getAll();

				const callCount = await actor.getOnStateChangeCallCount();
				// Only the one setValue should have triggered onStateChange
				expect(callCount).toBe(1);
			});

			test("state mutation in onStateChange returns state_mutation_reentrant", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const actor =
					client.stateChangeReentrantMutationActor.getOrCreate();

				await actor.setValue(5);

				const result = await actor.getResult();
				expect(result.callCount).toBe(1);
				expect(result.value).toBe(5);
				expect(result.derivedValue).toBe(0);
				expect(result.errorGroup).toBe("actor");
				expect(result.errorCode).toBe("state_mutation_reentrant");
			});
		});
	});
});
