import { describe, expect, test } from "vitest";
import type { InspectorSchedule } from "./actor-inspector-context";
import { formatDuration, formatSchedule } from "./actor-schedules-format";

const baseSchedule: InspectorSchedule = {
	id: "refresh-cache",
	name: "refresh-cache",
	kind: "every",
	action: "refresh",
	args: [],
	nextRunAt: 1_700_000_000_000,
};

describe("schedule inspector formatting", () => {
	test("formats one-time, interval, and cron schedules", () => {
		expect(
			formatSchedule({ ...baseSchedule, kind: "at", name: undefined }),
		).toBe("One time");
		expect(
			formatSchedule({ ...baseSchedule, intervalMs: 5 * 60_000 }),
		).toBe("Every 5 minutes");
		expect(
			formatSchedule({
				...baseSchedule,
				kind: "cron",
				expression: "0 9 * * *",
				timezone: "America/Los_Angeles",
			}),
		).toBe("0 9 * * * · America/Los_Angeles");
	});

	test("formats useful sub-second through multi-day durations", () => {
		expect(formatDuration(124)).toBe("124 ms");
		expect(formatDuration(5_000)).toBe("5 seconds");
		expect(formatDuration(90_000)).toBe("1.5 minutes");
		expect(formatDuration(7_200_000)).toBe("2 hours");
		expect(formatDuration(172_800_000)).toBe("2 days");
	});
});
