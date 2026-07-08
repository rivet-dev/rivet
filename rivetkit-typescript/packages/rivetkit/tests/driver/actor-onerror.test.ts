import { describe, expect, test } from "vitest";
import {
	getActorOnErrorReports,
	resetActorOnErrorReports,
} from "../../fixtures/driver-test-suite/actor-onerror";
import { describeDriverMatrix } from "./shared-matrix";
import { setupDriverTest, waitFor } from "./shared-utils";

async function waitForReports(
	driverTestConfig: Parameters<typeof waitFor>[0],
	key: string,
	count: number,
) {
	for (let i = 0; i < 60; i++) {
		const reports = getActorOnErrorReports(key);
		if (reports.length >= count) return reports;
		await waitFor(driverTestConfig, 50);
	}
	return getActorOnErrorReports(key);
}

describeDriverMatrix("Actor onError", (driverTestConfig) => {
	describe("universal actor onError", () => {
		test("reports action errors to actor hook before registry hook without suppressing", async (c) => {
			const key = "onerror-action";
			resetActorOnErrorReports(key);
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.actorOnErrorActionActor.getOrCreate([key]);

			await expect(handle.failAction()).rejects.toThrow();

			const reports = await waitForReports(driverTestConfig, key, 2);
			expect(reports.map((report) => report.source)).toEqual([
				"actor",
				"registry",
			]);
			expect(reports[0]).toMatchObject({
				arm: "action",
				detail: "failAction",
				scheduled: false,
				errorMessage: "onError action boom",
				rawError: true,
			});
			await expect(handle.succeed()).resolves.toBe("ok");
		});

		test("reports scheduled action errors as scheduled action events", async (c) => {
			const key = "onerror-scheduled";
			resetActorOnErrorReports(key);
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.actorOnErrorActionActor.getOrCreate([key]);

			await handle.scheduleFailure(10);

			const reports = await waitForReports(driverTestConfig, key, 2);
			expect(reports[0]).toMatchObject({
				source: "actor",
				arm: "action",
				detail: "scheduledFailure",
				scheduled: true,
				errorMessage: "onError scheduled boom",
			});
		});

		test("reports request hook errors", async (c) => {
			const key = "onerror-request";
			resetActorOnErrorReports(key);
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.actorOnErrorHookActor.getOrCreate([key]);

			const response = await handle.fetch("/boom");
			expect(response.status).toBe(500);

			const reports = await waitForReports(driverTestConfig, key, 2);
			expect(reports[0]).toMatchObject({
				arm: "hook",
				detail: "onRequest",
				errorMessage: "onError request boom",
			});
		});

		test("reports startup hook errors with the hook name", async (c) => {
			const key = "onerror-startup";
			resetActorOnErrorReports(key);
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.actorOnErrorStartupActor.getOrCreate([key]);

			await expect(handle.ping()).rejects.toThrow();

			const reports = await waitForReports(driverTestConfig, key, 2);
			expect(reports[0]).toMatchObject({
				arm: "hook",
				detail: "onCreate",
				errorMessage: "onError startup boom",
			});
		});

		test("reports queue publish errors as queue events", async (c) => {
			const key = "onerror-queue";
			resetActorOnErrorReports(key);
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.actorOnErrorQueueActor.getOrCreate([key]);

			await expect(handle.send("fail", { value: 1 })).rejects.toThrow();

			const reports = await waitForReports(driverTestConfig, key, 2);
			expect(reports[0]).toMatchObject({
				arm: "queue",
				detail: "fail",
				errorMessage: "onError queue boom",
			});
		});

		test.skipIf(!driverTestConfig.useRealTimers)(
			"reports action timeout errors",
			async (c) => {
				const key = "onerror-timeout";
				resetActorOnErrorReports(key);
				const { client } = await setupDriverTest(c, driverTestConfig);
				const handle =
					client.actorOnErrorTimeoutActor.getOrCreate([key]);

				await expect(handle.timeoutAction()).rejects.toThrow(
					/timed out/i,
				);

				const reports = await waitForReports(driverTestConfig, key, 2);
				expect(reports[0]).toMatchObject({
					arm: "action",
					detail: "timeoutAction",
					scheduled: false,
				});
				expect(reports[0].errorMessage).toMatch(/timed out/i);
			},
		);

		test("reports run handler failures as fatal run events", async (c) => {
			const key = "onerror-run";
			resetActorOnErrorReports(key);
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.actorOnErrorRunActor.getOrCreate([key]);

			await handle.ping().catch(() => undefined);

			const reports = await waitForReports(driverTestConfig, key, 2);
			expect(reports[0]).toMatchObject({
				arm: "fatal",
				detail: "run",
				errorMessage: "onError run boom",
			});
		});

		test("reports state serialization failures at the action boundary", async (c) => {
			const key = "onerror-internal";
			resetActorOnErrorReports(key);
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.actorOnErrorActionActor.getOrCreate([key]);

			await expect(handle.breakPersist()).rejects.toThrow();

			const reports = await waitForReports(driverTestConfig, key, 2);
			expect(reports[0]).toMatchObject({
				arm: "action",
				detail: "breakPersist",
			});
		});

		test("swallows throwing and rejecting onError hooks", async (c) => {
			resetActorOnErrorReports("onerror-throwing");
			resetActorOnErrorReports("onerror-rejecting");
			const { client } = await setupDriverTest(c, driverTestConfig);

			const throwing =
				client.actorOnErrorThrowingHookActor.getOrCreate([
					"onerror-throwing",
				]);
			await expect(throwing.failAction()).rejects.toThrow();
			await expect(throwing.ping()).resolves.toBe("pong");

			const rejecting =
				client.actorOnErrorRejectingHookActor.getOrCreate([
					"onerror-rejecting",
				]);
			await expect(rejecting.failAction()).rejects.toThrow();
			await expect(rejecting.ping()).resolves.toBe("pong");

			const throwingReports = await waitForReports(
				driverTestConfig,
				"onerror-throwing",
				2,
			);
			const rejectingReports = await waitForReports(
				driverTestConfig,
				"onerror-rejecting",
				2,
			);
			expect(throwingReports[0]?.source).toBe("throwing-actor");
			expect(rejectingReports[0]?.source).toBe("rejecting-actor");
		});

		test("does not report actor.aborted run unwind", async (c) => {
			const key = "onerror-aborted";
			resetActorOnErrorReports(key);
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.actorOnErrorAbortRunActor.getOrCreate([key]);

			await handle.ping();
			await handle.destroySelf();
			await waitFor(driverTestConfig, 250);

			expect(getActorOnErrorReports(key)).toEqual([]);
		});
	});
});
