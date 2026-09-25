import { randomUUID } from "node:crypto";
import { describe, expect, test } from "vitest";
import type { registry } from "../../fixtures/driver-test-suite/registry-static";
import { createClient } from "../../src/client/mod";
import { describeDriverMatrix } from "./shared-matrix";
import { setupDriverTest } from "./shared-utils";
import { startTcpProxy } from "./tcp-proxy";

describeDriverMatrix("Actor Conn Reconnect", (driverTestConfig) => {
	describe("Reconnect", () => {
		test("a connection recovers from an outage and does not resend a call whose reply was lost", async (c) => {
			const {
				client: direct,
				endpoint,
				namespace,
				poolName,
			} = await setupDriverTest(c, driverTestConfig);
			const proxy = await startTcpProxy(endpoint);
			c.onTestFinished(() => proxy.close());
			const client = createClient<typeof registry>({
				endpoint: proxy.url,
				namespace,
				poolName,
				encoding: driverTestConfig.encoding,
				disableMetadataLookup: true,
			});
			c.onTestFinished(() => client.dispose());
			const key = [`reconnect-${randomUUID()}`];
			const observer = direct.heldReplyActor.getOrCreate(key).connect();
			const incremented = new Promise<void>((resolve) => {
				observer.once("incremented", () => resolve());
			});
			await observer.getCount();

			const conn = client.heldReplyActor.getOrCreate(key).connect();
			const lostReply = conn.incrementAndHold();
			await incremented;
			proxy.down();
			await expect(lostReply).rejects.toMatchObject({
				group: "client",
				code: "connection_lost",
			});

			await proxy.refusedAttempts(2);
			const queuedDuringOutage = conn.getCount();
			await proxy.refusedAttempts(3);
			proxy.up();

			expect(await queuedDuringOutage).toBe(1);
			expect(conn.connStatus).toBe("connected");
			await direct.heldReplyActor.getOrCreate(key).send("release", true);
		});

		test("a connection the actor rejects stops, and later calls fail at once with the reason", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const conn = client.rejectConnectionActor
				.getOrCreate([`reject-${randomUUID()}`], {
					params: { reject: true },
				})
				.connect();
			const stopped = new Promise<void>((resolve) => {
				conn.onStatusChange((status) => {
					if (status === "idle") resolve();
				});
			});
			await stopped;

			await expect(conn.ping()).rejects.toMatchObject({
				group: "user",
				code: "rejected",
			});
			await expect(conn.ping()).rejects.toMatchObject({
				group: "user",
				code: "rejected",
			});
			expect(conn.connStatus).toBe("idle");
		});
	});
});
