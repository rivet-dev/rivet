import { describe, expect, test } from "vitest";
import { describeDriverMatrix } from "./shared-matrix";
import { setupDriverTest } from "./shared-utils";

describeDriverMatrix(
	"Gateway Response Start Timeout",
	(driverTestConfig) => {
		describe("gateway response start timeout", () => {
			test("an action that outlives the timeout runs once, and the caller gets the timeout", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const actor = client.responseStartTimeoutActor.getOrCreate([
					"runs-once",
				]);

				// The action reached the actor, so its result is unknown and
				// neither the gateway nor the client may send it again.
				await expect(actor.waitForRelease()).rejects.toMatchObject({
					group: "guard",
					code: "gateway_response_start_timeout",
				});
				await actor.release();
				expect(await actor.runs()).toBe(1);
			});
		});
	},
	{
		runtimes: ["native"],
		encodings: ["bare"],
		sqliteBackends: ["remote"],
		config: { engine: { gatewayResponseStartTimeoutMs: 2_000 } },
	},
);
