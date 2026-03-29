import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { CronManager } from "../src/cron/cron-manager.js";
import type { ScheduleDriver, ScheduleEntry, ScheduleHandle } from "../src/cron/schedule-driver.js";
import type { CronEvent } from "../src/cron/types.js";

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

	/** Manually trigger the callback for a job. */
	async fire(id: string): Promise<void> {
		const entry = this.entries.get(id);
		if (!entry) throw new Error(`No scheduled entry for id=${id}`);
		await entry.callback();
	}
}

// ---------------------------------------------------------------------------
// Mock AgentOs — stubs for exec and createSession
// ---------------------------------------------------------------------------

function createMockVm() {
	return {
		exec: vi.fn().mockResolvedValue({ exitCode: 0, stdout: "", stderr: "" }),
		createSession: vi.fn().mockResolvedValue({ sessionId: "mock-session-1" }),
		prompt: vi.fn().mockResolvedValue(undefined),
		closeSession: vi.fn(),
	};
}

describe("CronManager", () => {
	let driver: MockScheduleDriver;
	let vm: ReturnType<typeof createMockVm>;
	let manager: CronManager;

	beforeEach(() => {
		driver = new MockScheduleDriver();
		vm = createMockVm();
		manager = new CronManager(vm as any, driver);
	});

	afterEach(() => {
		manager.dispose();
	});

	// -----------------------------------------------------------------------
	// Schedule & list
	// -----------------------------------------------------------------------

	it("schedule and list returns job info", () => {
		const job = manager.schedule({
			id: "j1",
			schedule: "* * * * *",
			action: { type: "callback", fn: () => {} },
		});

		expect(job.id).toBe("j1");

		const list = manager.list();
		expect(list).toHaveLength(1);
		expect(list[0].id).toBe("j1");
		expect(list[0].schedule).toBe("* * * * *");
		expect(list[0].overlap).toBe("allow");
		expect(list[0].runCount).toBe(0);
		expect(list[0].running).toBe(false);
	});

	// -----------------------------------------------------------------------
	// Cancel
	// -----------------------------------------------------------------------

	it("cancel removes job from list", () => {
		const job = manager.schedule({
			id: "j2",
			schedule: "* * * * *",
			action: { type: "callback", fn: () => {} },
		});

		job.cancel();

		expect(manager.list()).toHaveLength(0);
		// Also removed from driver
		expect(driver.entries.has("j2")).toBe(false);
	});

	// -----------------------------------------------------------------------
	// Callback action
	// -----------------------------------------------------------------------

	it("callback action is invoked when driver fires", async () => {
		const fn = vi.fn();
		manager.schedule({
			id: "j3",
			schedule: "* * * * *",
			action: { type: "callback", fn },
		});

		await driver.fire("j3");

		expect(fn).toHaveBeenCalledTimes(1);
	});

	// -----------------------------------------------------------------------
	// Exec action
	// -----------------------------------------------------------------------

	it("exec action calls vm.exec with correct command and args", async () => {
		manager.schedule({
			id: "j4",
			schedule: "* * * * *",
			action: { type: "exec", command: "echo", args: ["hello", "world"] },
		});

		await driver.fire("j4");

		expect(vm.exec).toHaveBeenCalledTimes(1);
		expect(vm.exec).toHaveBeenCalledWith("echo hello world");
	});

	// -----------------------------------------------------------------------
	// Session action
	// -----------------------------------------------------------------------

	it("session action calls vm.createSession, session.prompt, session.close", async () => {
		manager.schedule({
			id: "j5",
			schedule: "* * * * *",
			action: {
				type: "session",
				agentType: "pi" as any,
				prompt: "do something",
			},
		});

		await driver.fire("j5");

		expect(vm.createSession).toHaveBeenCalledTimes(1);
		expect(vm.createSession).toHaveBeenCalledWith("pi", undefined);
		expect(vm.prompt).toHaveBeenCalledWith("mock-session-1", "do something");
		expect(vm.closeSession).toHaveBeenCalledWith("mock-session-1");
	});

	// -----------------------------------------------------------------------
	// Overlap: skip
	// -----------------------------------------------------------------------

	it("overlap 'skip' drops execution when previous still running", async () => {
		let resolveFirst!: () => void;
		const firstCallPromise = new Promise<void>((resolve) => {
			resolveFirst = resolve;
		});
		let callCount = 0;

		manager.schedule({
			id: "j6",
			schedule: "* * * * *",
			action: {
				type: "callback",
				fn: () => {
					callCount++;
					if (callCount === 1) return firstCallPromise;
				},
			},
			overlap: "skip",
		});

		// Start first execution (it will hang on the promise)
		const firstExec = driver.fire("j6");

		// Fire again while first is still running — should be skipped
		await driver.fire("j6");

		// Resolve first
		resolveFirst();
		await firstExec;

		expect(callCount).toBe(1);
	});

	// -----------------------------------------------------------------------
	// Overlap: queue
	// -----------------------------------------------------------------------

	it("overlap 'queue' waits for previous then runs", async () => {
		const executionOrder: number[] = [];
		let resolveFirst!: () => void;
		const firstCallPromise = new Promise<void>((resolve) => {
			resolveFirst = resolve;
		});
		let callCount = 0;

		manager.schedule({
			id: "j7",
			schedule: "* * * * *",
			action: {
				type: "callback",
				fn: () => {
					callCount++;
					executionOrder.push(callCount);
					if (callCount === 1) return firstCallPromise;
				},
			},
			overlap: "queue",
		});

		// Start first execution (hangs)
		const firstExec = driver.fire("j7");

		// Fire again while first is running — should be queued
		await driver.fire("j7");

		// Only 1 execution so far
		expect(executionOrder).toEqual([1]);

		// Resolve first — queued execution should start
		resolveFirst();
		await firstExec;

		// Allow queued microtask to resolve
		await new Promise((r) => setTimeout(r, 0));

		expect(executionOrder).toEqual([1, 2]);
	});

	// -----------------------------------------------------------------------
	// Overlap: allow (default)
	// -----------------------------------------------------------------------

	it("overlap 'allow' runs concurrently (default)", async () => {
		let resolveFirst!: () => void;
		const firstCallPromise = new Promise<void>((resolve) => {
			resolveFirst = resolve;
		});
		let resolveSecond!: () => void;
		const secondCallPromise = new Promise<void>((resolve) => {
			resolveSecond = resolve;
		});
		let callCount = 0;

		manager.schedule({
			id: "j8",
			schedule: "* * * * *",
			action: {
				type: "callback",
				fn: () => {
					callCount++;
					if (callCount === 1) return firstCallPromise;
					if (callCount === 2) return secondCallPromise;
				},
			},
		});

		// Start first execution
		const firstExec = driver.fire("j8");
		// Start second concurrently
		const secondExec = driver.fire("j8");

		// Both started
		expect(callCount).toBe(2);

		resolveFirst();
		resolveSecond();
		await firstExec;
		await secondExec;
	});

	// -----------------------------------------------------------------------
	// Events: cron:fire
	// -----------------------------------------------------------------------

	it("cron:fire event emitted before execution", async () => {
		const events: CronEvent[] = [];
		manager.onEvent((e) => events.push(e));

		manager.schedule({
			id: "j9",
			schedule: "* * * * *",
			action: { type: "callback", fn: () => {} },
		});

		await driver.fire("j9");

		const fireEvent = events.find((e) => e.type === "cron:fire");
		expect(fireEvent).toBeDefined();
		expect(fireEvent!.jobId).toBe("j9");
	});

	// -----------------------------------------------------------------------
	// Events: cron:complete
	// -----------------------------------------------------------------------

	it("cron:complete event emitted with durationMs after success", async () => {
		const events: CronEvent[] = [];
		manager.onEvent((e) => events.push(e));

		manager.schedule({
			id: "j10",
			schedule: "* * * * *",
			action: { type: "callback", fn: () => {} },
		});

		await driver.fire("j10");

		const complete = events.find((e) => e.type === "cron:complete");
		expect(complete).toBeDefined();
		expect(complete!.jobId).toBe("j10");
		expect(complete!.type === "cron:complete" && complete!.durationMs).toBeGreaterThanOrEqual(0);
	});

	// -----------------------------------------------------------------------
	// Events: cron:error
	// -----------------------------------------------------------------------

	it("cron:error event emitted when action throws (manager continues running)", async () => {
		const events: CronEvent[] = [];
		manager.onEvent((e) => events.push(e));

		const error = new Error("boom");
		manager.schedule({
			id: "j11",
			schedule: "* * * * *",
			action: {
				type: "callback",
				fn: () => {
					throw error;
				},
			},
		});

		// Should not throw
		await driver.fire("j11");

		const errorEvent = events.find((e) => e.type === "cron:error");
		expect(errorEvent).toBeDefined();
		expect(errorEvent!.jobId).toBe("j11");
		expect(errorEvent!.type === "cron:error" && errorEvent!.error).toBe(error);

		// Manager still functional — can schedule new jobs
		const fn2 = vi.fn();
		manager.schedule({
			id: "j11b",
			schedule: "* * * * *",
			action: { type: "callback", fn: fn2 },
		});
		await driver.fire("j11b");
		expect(fn2).toHaveBeenCalledTimes(1);
	});

	// -----------------------------------------------------------------------
	// Dispose
	// -----------------------------------------------------------------------

	it("dispose cancels all jobs", () => {
		manager.schedule({
			id: "j12a",
			schedule: "* * * * *",
			action: { type: "callback", fn: () => {} },
		});
		manager.schedule({
			id: "j12b",
			schedule: "* * * * *",
			action: { type: "callback", fn: () => {} },
		});

		manager.dispose();

		expect(manager.list()).toHaveLength(0);
		expect(driver.disposed).toBe(true);
	});
});
