import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { setLongTimeout } from "../src/utils";

describe("setLongTimeout", () => {
	beforeEach(() => {
		vi.useFakeTimers();
	});

	afterEach(() => {
		vi.restoreAllMocks();
	});

	test("executes callback for short timeouts", () => {
		const callback = vi.fn();
		const handle = setLongTimeout(callback, 100);

		expect(callback).not.toHaveBeenCalled();

		vi.advanceTimersByTime(99);
		expect(callback).not.toHaveBeenCalled();

		vi.advanceTimersByTime(1);
		expect(callback).toHaveBeenCalledTimes(1);
	});

	test("executes callback for zero timeout", () => {
		const callback = vi.fn();
		setLongTimeout(callback, 0);

		expect(callback).not.toHaveBeenCalled();

		vi.advanceTimersByTime(0);
		expect(callback).toHaveBeenCalledTimes(1);
	});

	test("executes callback for negative timeout (treated as 0)", () => {
		const callback = vi.fn();
		setLongTimeout(callback, -100);

		expect(callback).not.toHaveBeenCalled();

		vi.advanceTimersByTime(0);
		expect(callback).toHaveBeenCalledTimes(1);
	});

	test("handles timeout at exactly TIMEOUT_MAX", () => {
		const callback = vi.fn();
		const TIMEOUT_MAX = 2147483647;
		setLongTimeout(callback, TIMEOUT_MAX);

		expect(callback).not.toHaveBeenCalled();

		vi.advanceTimersByTime(TIMEOUT_MAX - 1);
		expect(callback).not.toHaveBeenCalled();

		vi.advanceTimersByTime(1);
		expect(callback).toHaveBeenCalledTimes(1);
	});

	test("handles timeout larger than TIMEOUT_MAX", () => {
		const callback = vi.fn();
		const TIMEOUT_MAX = 2147483647;
		const longTimeout = TIMEOUT_MAX + 1000;

		setLongTimeout(callback, longTimeout);

		expect(callback).not.toHaveBeenCalled();

		// Advance to just before TIMEOUT_MAX
		vi.advanceTimersByTime(TIMEOUT_MAX - 1);
		expect(callback).not.toHaveBeenCalled();

		// Advance past TIMEOUT_MAX - should trigger intermediate timeout
		vi.advanceTimersByTime(1);
		expect(callback).not.toHaveBeenCalled();

		// Advance the remaining time
		vi.advanceTimersByTime(999);
		expect(callback).not.toHaveBeenCalled();

		vi.advanceTimersByTime(1);
		expect(callback).toHaveBeenCalledTimes(1);
	});

	test("handles very large timeout requiring multiple chunks", () => {
		const callback = vi.fn();
		const TIMEOUT_MAX = 2147483647;
		const veryLongTimeout = TIMEOUT_MAX * 2 + 5000;

		setLongTimeout(callback, veryLongTimeout);

		expect(callback).not.toHaveBeenCalled();

		// First chunk
		vi.advanceTimersByTime(TIMEOUT_MAX);
		expect(callback).not.toHaveBeenCalled();

		// Second chunk
		vi.advanceTimersByTime(TIMEOUT_MAX);
		expect(callback).not.toHaveBeenCalled();

		// Final remainder
		vi.advanceTimersByTime(4999);
		expect(callback).not.toHaveBeenCalled();

		vi.advanceTimersByTime(1);
		expect(callback).toHaveBeenCalledTimes(1);
	});

	test("abort cancels short timeout", () => {
		const callback = vi.fn();
		const handle = setLongTimeout(callback, 100);

		vi.advanceTimersByTime(50);
		expect(callback).not.toHaveBeenCalled();

		handle.abort();

		vi.advanceTimersByTime(100);
		expect(callback).not.toHaveBeenCalled();
	});

	test("abort cancels long timeout during first chunk", () => {
		const callback = vi.fn();
		const TIMEOUT_MAX = 2147483647;
		const handle = setLongTimeout(callback, TIMEOUT_MAX + 1000);

		vi.advanceTimersByTime(1000);
		expect(callback).not.toHaveBeenCalled();

		handle.abort();

		vi.advanceTimersByTime(TIMEOUT_MAX + 1000);
		expect(callback).not.toHaveBeenCalled();
	});

	test("abort cancels long timeout after first chunk", () => {
		const callback = vi.fn();
		const TIMEOUT_MAX = 2147483647;
		const handle = setLongTimeout(callback, TIMEOUT_MAX + 1000);

		// Advance past first chunk
		vi.advanceTimersByTime(TIMEOUT_MAX);
		expect(callback).not.toHaveBeenCalled();

		// Abort during second chunk
		handle.abort();

		vi.advanceTimersByTime(1000);
		expect(callback).not.toHaveBeenCalled();
	});

	test("multiple abort calls are safe", () => {
		const callback = vi.fn();
		const handle = setLongTimeout(callback, 100);

		handle.abort();
		handle.abort(); // Second abort should not throw

		vi.advanceTimersByTime(100);
		expect(callback).not.toHaveBeenCalled();
	});

	test("abort after timeout has fired is safe", () => {
		const callback = vi.fn();
		const handle = setLongTimeout(callback, 100);

		vi.advanceTimersByTime(100);
		expect(callback).toHaveBeenCalledTimes(1);

		// Abort after callback has executed should not throw
		handle.abort();

		vi.advanceTimersByTime(100);
		expect(callback).toHaveBeenCalledTimes(1); // Still only called once
	});

	test("handles multiple concurrent timeouts", () => {
		const callback1 = vi.fn();
		const callback2 = vi.fn();
		const callback3 = vi.fn();

		setLongTimeout(callback1, 50);
		setLongTimeout(callback2, 100);
		setLongTimeout(callback3, 150);

		vi.advanceTimersByTime(49);
		expect(callback1).not.toHaveBeenCalled();
		expect(callback2).not.toHaveBeenCalled();
		expect(callback3).not.toHaveBeenCalled();

		vi.advanceTimersByTime(1); // 50ms total
		expect(callback1).toHaveBeenCalledTimes(1);
		expect(callback2).not.toHaveBeenCalled();
		expect(callback3).not.toHaveBeenCalled();

		vi.advanceTimersByTime(50); // 100ms total
		expect(callback1).toHaveBeenCalledTimes(1);
		expect(callback2).toHaveBeenCalledTimes(1);
		expect(callback3).not.toHaveBeenCalled();

		vi.advanceTimersByTime(50); // 150ms total
		expect(callback1).toHaveBeenCalledTimes(1);
		expect(callback2).toHaveBeenCalledTimes(1);
		expect(callback3).toHaveBeenCalledTimes(1);
	});

	test("real-world alarm scenario - 100ms delay", () => {
		const callback = vi.fn();
		const now = Date.now();
		const futureTimestamp = now + 100;
		const delay = Math.max(0, futureTimestamp - now);

		expect(delay).toBe(100);

		setLongTimeout(callback, delay);

		vi.advanceTimersByTime(99);
		expect(callback).not.toHaveBeenCalled();

		vi.advanceTimersByTime(1);
		expect(callback).toHaveBeenCalledTimes(1);
	});

	test("handles async callbacks", async () => {
		const asyncCallback = vi.fn(async () => {
			await Promise.resolve();
			return "done";
		});

		setLongTimeout(asyncCallback, 100);

		vi.advanceTimersByTime(100);
		expect(asyncCallback).toHaveBeenCalledTimes(1);
	});

	test("callback receives no arguments", () => {
		const callback = vi.fn();
		setLongTimeout(callback, 100);

		vi.advanceTimersByTime(100);
		expect(callback).toHaveBeenCalledWith();
	});
});
