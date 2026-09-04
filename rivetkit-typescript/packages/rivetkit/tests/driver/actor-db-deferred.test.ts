import { expect, test } from "vitest";
import {
	describeDriverMatrix,
	SQLITE_DRIVER_MATRIX_OPTIONS,
} from "./shared-matrix";
import { setupDriverTest } from "./shared-utils";

describeDriverMatrix(
	"Actor database deferred commits",
	(driverTestConfig) => {
		const supported =
			driverTestConfig.runtime === "native" &&
			driverTestConfig.sqliteBackend === "local";

		if (!supported) {
			test("rejects deferred commits on non-local runtimes", async (c) => {
				const { client, getRuntimeOutput } = await setupDriverTest(
					c,
					driverTestConfig,
				);
				const instance = client.dbActorDeferred.getOrCreate([
					"unsupported-deferred",
				]);
				await expect(instance.ready).rejects.toMatchObject({
					code: "actor_wake_retries_exceeded",
				});
				expect(getRuntimeOutput()).toContain(
					"deferred_commits_unsupported: Deferred SQLite commits are unsupported.",
				);
			});
			return;
		}

		test("reads local writes before and after an explicit flush", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const key = ["visibility"];
			const instance = client.sleepDbActorDeferred
				.getOrCreate(key)
				.connect();
			await instance.ready;
			const rows = await instance.writeAndRead("visible");
			expect(rows).toEqual([{ value: "visible" }]);
			const progress = await instance.waitForFlush();
			expect(progress.flushed).toBeGreaterThanOrEqual(progress.commit);
			await instance.triggerSleep();
			await instance.dispose();
			const reopened = client.sleepDbActorDeferred.getOrCreate(key);
			expect(await reopened.values()).toEqual([{ value: "visible" }]);
			expect(await reopened.sequence()).toBeGreaterThan(progress.commit);
		});

		test("advances commit sequence for writes but not reads", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const instance = client.dbActorDeferred.getOrCreate(["sequences"]);
			const write = await instance.write("one");
			expect(write.after).toBeGreaterThan(write.before);
			const read = await instance.readSequence();
			expect(read.after).toBe(read.before);
			const flushed = await instance.waitForFlush(write.after);
			expect(flushed.flushed).toBeGreaterThanOrEqual(write.after);
			const snapshot = await instance.waitSnapshotThenWrite("two");
			expect(snapshot.after).toBeGreaterThan(snapshot.captured);
			expect(snapshot.flushedAtEarlierWait).toBeLessThan(
				snapshot.after ?? 0,
			);
		});

		test("reports readonly metadata and supports a turn-spanning handle", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const instance = client.dbActorDeferred.getOrCreate(["handle"]);
			expect(await instance.readonlyMetadata()).toEqual({
				select: true,
				insert: false,
				ddl: false,
			});
			const result = await instance.handleTransaction("turn");
			expect(result.baseSyncError).toContain(
				"Use the open synchronous transaction handle",
			);
			expect(result.isOpen).toBe(false);
			expect(result.resolvedWhileOpen).toBe(false);
			expect(result.count).toBe(2);
			expect(result.committedSequence).toBeTypeOf("number");
			expect(result.committedSequence).toBe(result.sequence);
			expect(result.sequence).toBeGreaterThan(0);
			expect(await instance.readOnlyHandle()).toBeNull();
		});

		test("survives a sleep after flushing", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const key = ["sleep"];
			const instance = client.sleepDbActorDeferred
				.getOrCreate(key)
				.connect();
			await instance.ready;
			const before = await instance.writeAndFlush("durable");
			await instance.triggerSleep();
			await instance.dispose();
			const woke = client.sleepDbActorDeferred.getOrCreate(key);
			expect(await woke.values()).toEqual([{ value: "durable" }]);
			expect(await woke.sequence()).toBeGreaterThan(before);
		});
	},
	SQLITE_DRIVER_MATRIX_OPTIONS,
);
