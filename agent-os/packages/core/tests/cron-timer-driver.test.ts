import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { TimerScheduleDriver } from "../src/cron/timer-driver.js";
import type { ScheduleEntry } from "../src/cron/schedule-driver.js";

describe("TimerScheduleDriver", () => {
	let driver: TimerScheduleDriver;

	beforeEach(() => {
		vi.useFakeTimers();
		// Set a known base time: 2026-01-01T00:00:00Z
		vi.setSystemTime(new Date("2026-01-01T00:00:00Z"));
		driver = new TimerScheduleDriver();
	});

	afterEach(() => {
		driver.dispose();
		vi.useRealTimers();
	});

	it("schedule with cron expression fires callback at computed next time", async () => {
		const callback = vi.fn();
		// Every minute
		driver.schedule({ id: "job-1", schedule: "* * * * *", callback });

		// Advance to just before the next minute mark
		await vi.advanceTimersByTimeAsync(59_999);
		expect(callback).not.toHaveBeenCalled();

		// Advance past the minute mark
		await vi.advanceTimersByTimeAsync(1);
		expect(callback).toHaveBeenCalledTimes(1);
	});

	it("recurring cron reschedules after each fire", async () => {
		const callback = vi.fn();
		// Every minute
		driver.schedule({ id: "job-2", schedule: "* * * * *", callback });

		// Fire first time at T+60s
		await vi.advanceTimersByTimeAsync(60_000);
		expect(callback).toHaveBeenCalledTimes(1);

		// Fire second time at T+120s
		await vi.advanceTimersByTimeAsync(60_000);
		expect(callback).toHaveBeenCalledTimes(2);

		// Fire third time at T+180s
		await vi.advanceTimersByTimeAsync(60_000);
		expect(callback).toHaveBeenCalledTimes(3);
	});

	it("one-shot ISO timestamp fires once and does not reschedule", async () => {
		const callback = vi.fn();
		// 5 seconds in the future
		driver.schedule({
			id: "job-3",
			schedule: "2026-01-01T00:00:05Z",
			callback,
		});

		await vi.advanceTimersByTimeAsync(5_000);
		expect(callback).toHaveBeenCalledTimes(1);

		// Advance much further; should not fire again
		await vi.advanceTimersByTimeAsync(60_000);
		expect(callback).toHaveBeenCalledTimes(1);
	});

	it("cancel prevents pending callback from firing", async () => {
		const callback = vi.fn();
		const handle = driver.schedule({
			id: "job-4",
			schedule: "* * * * *",
			callback,
		});

		// Cancel before the first fire
		driver.cancel(handle);

		await vi.advanceTimersByTimeAsync(120_000);
		expect(callback).not.toHaveBeenCalled();
	});

	it("dispose clears all pending timers", async () => {
		const callback1 = vi.fn();
		const callback2 = vi.fn();
		driver.schedule({ id: "job-5a", schedule: "* * * * *", callback: callback1 });
		driver.schedule({ id: "job-5b", schedule: "* * * * *", callback: callback2 });

		driver.dispose();

		await vi.advanceTimersByTimeAsync(120_000);
		expect(callback1).not.toHaveBeenCalled();
		expect(callback2).not.toHaveBeenCalled();
	});

	it("schedule with past ISO timestamp fires immediately (delay clamped to 0)", async () => {
		const callback = vi.fn();
		// 10 seconds in the past
		driver.schedule({
			id: "job-6",
			schedule: "2025-12-31T23:59:50Z",
			callback,
		});

		// Should fire on next tick (delay clamped to 0)
		await vi.advanceTimersByTimeAsync(0);
		expect(callback).toHaveBeenCalledTimes(1);

		// Should not reschedule (one-shot)
		await vi.advanceTimersByTimeAsync(60_000);
		expect(callback).toHaveBeenCalledTimes(1);
	});

	it("multiple concurrent schedules fire independently", async () => {
		const callback1 = vi.fn();
		const callback2 = vi.fn();

		// Every minute
		driver.schedule({ id: "job-7a", schedule: "* * * * *", callback: callback1 });
		// 30 seconds from now
		driver.schedule({
			id: "job-7b",
			schedule: "2026-01-01T00:00:30Z",
			callback: callback2,
		});

		// At T+30s: only the one-shot fires
		await vi.advanceTimersByTimeAsync(30_000);
		expect(callback1).not.toHaveBeenCalled();
		expect(callback2).toHaveBeenCalledTimes(1);

		// At T+60s: the cron fires
		await vi.advanceTimersByTimeAsync(30_000);
		expect(callback1).toHaveBeenCalledTimes(1);
		expect(callback2).toHaveBeenCalledTimes(1); // still 1 (one-shot)

		// At T+120s: cron fires again
		await vi.advanceTimersByTimeAsync(60_000);
		expect(callback1).toHaveBeenCalledTimes(2);
		expect(callback2).toHaveBeenCalledTimes(1);
	});
});
