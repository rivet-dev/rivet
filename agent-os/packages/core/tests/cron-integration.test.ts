import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type {
	ScheduleDriver,
	ScheduleEntry,
	ScheduleHandle,
} from "../src/cron/schedule-driver.js";
import type { CronEvent } from "../src/cron/types.js";
import { AgentOs } from "../src/index.js";

// ---------------------------------------------------------------------------
// Mock ScheduleDriver — stores callbacks and fires them on demand
// ---------------------------------------------------------------------------

class MockScheduleDriver implements ScheduleDriver {
	entries = new Map<string, ScheduleEntry>();
	disposed = false;

	schedule(entry: ScheduleEntry): ScheduleHandle {
		this.entries.set(entry.id, entry);
		return { id: entry.id };
	}

	cancel(handle: ScheduleHandle): void {
		this.entries.delete(handle.id);
	}

	dispose(): void {
		this.entries.clear();
		this.disposed = true;
	}

	async fire(id: string): Promise<void> {
		const entry = this.entries.get(id);
		if (!entry) throw new Error(`No scheduled entry for id=${id}`);
		await entry.callback();
	}
}

// ---------------------------------------------------------------------------
// WASM commands directory (needed for exec action tests)
// ---------------------------------------------------------------------------

import {
	REGISTRY_SOFTWARE,
	hasRegistryCommands,
} from "./helpers/registry-commands.js";

describe("cron integration via AgentOs API", () => {
	let driver: MockScheduleDriver;
	let vm: AgentOs;

	beforeEach(async () => {
		driver = new MockScheduleDriver();
		vm = await AgentOs.create({
			scheduleDriver: driver,
			...(hasRegistryCommands ? { software: REGISTRY_SOFTWARE } : {}),
		});
	});

	afterEach(async () => {
		await vm.dispose();
	});

	it.skipIf(!hasRegistryCommands)(
		"scheduleCron with exec action writes file inside VM on schedule",
		async () => {
			vm.scheduleCron({
				id: "exec-job",
				schedule: "* * * * *",
				action: {
					type: "exec",
					command: "echo cron-wrote-this > /tmp/cron-marker",
				},
			});

			await driver.fire("exec-job");

			const data = await vm.readFile("/tmp/cron-marker");
			const text = new TextDecoder().decode(data);
			expect(text).toContain("cron-wrote-this");
		},
	);

	it("scheduleCron with callback action invokes function", async () => {
		const fn = vi.fn();
		vm.scheduleCron({
			id: "cb-job",
			schedule: "* * * * *",
			action: { type: "callback", fn },
		});

		await driver.fire("cb-job");

		expect(fn).toHaveBeenCalledTimes(1);
	});

	it("listCronJobs returns scheduled job with correct info", () => {
		vm.scheduleCron({
			id: "list-job",
			schedule: "*/5 * * * *",
			action: { type: "callback", fn: () => {} },
			overlap: "skip",
		});

		const jobs = vm.listCronJobs();
		expect(jobs).toHaveLength(1);
		expect(jobs[0].id).toBe("list-job");
		expect(jobs[0].schedule).toBe("*/5 * * * *");
		expect(jobs[0].overlap).toBe("skip");
		expect(jobs[0].runCount).toBe(0);
		expect(jobs[0].running).toBe(false);
	});

	it("cancelCronJob stops future executions", () => {
		const fn = vi.fn();
		vm.scheduleCron({
			id: "cancel-job",
			schedule: "* * * * *",
			action: { type: "callback", fn },
		});

		vm.cancelCronJob("cancel-job");

		expect(driver.entries.has("cancel-job")).toBe(false);
		expect(vm.listCronJobs()).toHaveLength(0);
	});

	it("onCronEvent receives cron:complete after successful execution", async () => {
		const events: CronEvent[] = [];
		vm.onCronEvent((e) => events.push(e));

		vm.scheduleCron({
			id: "event-ok-job",
			schedule: "* * * * *",
			action: { type: "callback", fn: () => {} },
		});

		await driver.fire("event-ok-job");

		const complete = events.find((e) => e.type === "cron:complete");
		expect(complete).toBeDefined();
		expect(complete?.jobId).toBe("event-ok-job");
		if (complete?.type === "cron:complete") {
			expect(complete.durationMs).toBeGreaterThanOrEqual(0);
		}
	});

	it("onCronEvent receives cron:error when action fails", async () => {
		const events: CronEvent[] = [];
		vm.onCronEvent((e) => events.push(e));

		const error = new Error("cron-boom");
		vm.scheduleCron({
			id: "event-err-job",
			schedule: "* * * * *",
			action: {
				type: "callback",
				fn: () => {
					throw error;
				},
			},
		});

		await driver.fire("event-err-job");

		const errEvent = events.find((e) => e.type === "cron:error");
		expect(errEvent).toBeDefined();
		expect(errEvent?.jobId).toBe("event-err-job");
		if (errEvent?.type === "cron:error") {
			expect(errEvent.error).toBe(error);
		}
	});

	it("dispose cancels all cron jobs (no timers leak)", async () => {
		vm.scheduleCron({
			id: "dispose-1",
			schedule: "* * * * *",
			action: { type: "callback", fn: () => {} },
		});
		vm.scheduleCron({
			id: "dispose-2",
			schedule: "* * * * *",
			action: { type: "callback", fn: () => {} },
		});

		await vm.dispose();

		expect(driver.disposed).toBe(true);
		expect(driver.entries.size).toBe(0);
	});
});

describe("custom ScheduleDriver via AgentOsOptions", () => {
	it("custom driver receives schedule and cancel calls instead of default timer", async () => {
		const customDriver = new MockScheduleDriver();
		const vm = await AgentOs.create({ scheduleDriver: customDriver });

		const job = vm.scheduleCron({
			id: "custom-job",
			schedule: "* * * * *",
			action: { type: "callback", fn: () => {} },
		});

		expect(customDriver.entries.has("custom-job")).toBe(true);

		job.cancel();
		expect(customDriver.entries.has("custom-job")).toBe(false);

		await vm.dispose();
		expect(customDriver.disposed).toBe(true);
	});
});
