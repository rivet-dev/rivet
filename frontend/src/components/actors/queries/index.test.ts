import { describe, it, expect } from "vitest";
import { getActorStatus } from "./index";

// createTs is always set for all test cases.
const base = {
	createTs: 1000,
	connectableTs: undefined,
	destroyTs: undefined,
	sleepTs: undefined,
	pendingAllocationTs: undefined,
	rescheduleTs: undefined,
	error: undefined,
};

describe("getActorStatus", () => {
	it("returns 'starting' when no other timestamps are set", () => {
		expect(getActorStatus({ ...base })).toBe("starting");
	});

	it("returns 'running' when connectableTs is set", () => {
		expect(getActorStatus({ ...base, connectableTs: 2000 })).toBe(
			"running",
		);
	});

	it("returns 'stopped' when destroyTs is set", () => {
		expect(
			getActorStatus({ ...base, connectableTs: 2000, destroyTs: 3000 }),
		).toBe("stopped");
	});

	it("returns 'crashed' when error is set", () => {
		expect(
			getActorStatus({ ...base, error: { message: "out of memory" } }),
		).toBe("crashed");
	});

	it("returns 'sleeping' when sleepTs is set", () => {
		expect(getActorStatus({ ...base, sleepTs: 2000 })).toBe("sleeping");
	});

	it("returns 'pending' when pendingAllocationTs is set", () => {
		expect(getActorStatus({ ...base, pendingAllocationTs: 2000 })).toBe(
			"pending",
		);
	});

	it("returns 'crash-loop' when rescheduleTs is set", () => {
		expect(getActorStatus({ ...base, rescheduleTs: 2000 })).toBe(
			"crash-loop",
		);
	});

	// Priority edge cases
	it("returns 'running' even when error is set (running takes priority)", () => {
		expect(
			getActorStatus({
				...base,
				connectableTs: 2000,
				error: { message: "something" },
			}),
		).toBe("running");
	});

	it("returns 'crashed' over 'pending' when both error and pendingAllocationTs are set", () => {
		expect(
			getActorStatus({
				...base,
				pendingAllocationTs: 2000,
				error: { message: "failed to start" },
			}),
		).toBe("crashed");
	});

	it("returns 'crashed' when destroyed without ever being connectable", () => {
		expect(getActorStatus({ ...base, destroyTs: 2000 })).toBe("crashed");
	});

	it("returns 'unknown' when createTs is not set", () => {
		expect(
			getActorStatus({
				...base,
				createTs: undefined,
			} as unknown as Parameters<typeof getActorStatus>[0]),
		).toBe("unknown");
	});
});
