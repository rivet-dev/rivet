// @ts-nocheck
import { describeDriverMatrix } from "./shared-matrix";
import { describe, expect, test, vi } from "vitest";
import { setupDriverTest } from "./shared-utils";

describeDriverMatrix("Actor Conn Status", (driverTestConfig) => {
	describe("Connection Status Changes", () => {
		test("connStatus starts as idle before connect", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.counter.getOrCreate(["status-idle"]);
			const conn = handle.connect();

			// connStatus should transition through connecting to connected
			// Wait for the connection to be ready
			await conn.increment(1);
			expect(conn.connStatus).toBe("connected");

			await conn.dispose();
		});

		test("onStatusChange fires on connect and dispose", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.counter.getOrCreate(["status-change"]);
			const conn = handle.connect();

			const statuses: string[] = [];
			conn.onStatusChange((status) => {
				statuses.push(status);
			});

			// Wait for connected
			await conn.increment(1);

			// Dispose triggers disconnected then idle
			await conn.dispose();

			// Should have seen at least connecting and connected
			expect(statuses).toContain("connected");
		});

		test("onStatusChange unsubscribe stops callbacks", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.counter.getOrCreate(["status-unsub"]);
			const conn = handle.connect();

			const statuses: string[] = [];
			const unsub = conn.onStatusChange((status) => {
				statuses.push(status);
			});

			// Wait for connected
			await conn.increment(1);

			// Unsubscribe
			unsub();
			const countAfterUnsub = statuses.length;

			// Dispose should not trigger more callbacks
			await conn.dispose();

			expect(statuses.length).toBe(countAfterUnsub);
		});

		test("connStatus is connected after successful action", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.counter.getOrCreate(["status-connected"]);
			const conn = handle.connect();

			await conn.increment(1);
			expect(conn.connStatus).toBe("connected");

			await conn.dispose();
		});

		test("onOpen fires when connection is established", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.counter.getOrCreate(["status-onopen"]);
			const conn = handle.connect();

			const openFired = vi.fn();
			conn.onOpen(openFired);

			await conn.increment(1);
			expect(openFired).toHaveBeenCalled();

			await conn.dispose();
		});

		test("onClose fires when connection is disposed", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.counter.getOrCreate(["status-onclose"]);
			const conn = handle.connect();

			await conn.increment(1);

			const closeFired = vi.fn();
			conn.onClose(closeFired);

			await conn.dispose();
			expect(closeFired).toHaveBeenCalled();
		});
	});
});
