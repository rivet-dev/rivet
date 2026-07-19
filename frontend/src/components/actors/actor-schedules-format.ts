import cronstrue from "cronstrue";
import type { InspectorSchedule } from "./actor-inspector-context";

export function formatSchedule(schedule: InspectorSchedule): string {
	if (schedule.kind === "at") {
		return new Intl.DateTimeFormat(undefined, {
			dateStyle: "medium",
			timeStyle: "short",
		}).format(schedule.nextRunAt);
	}
	if (schedule.kind === "every") {
		return `Every ${formatDuration(schedule.intervalMs ?? 0)}`;
	}
	return schedule.expression ?? "Cron";
}

export function describeCronExpression(expression: string): string {
	try {
		return cronstrue.toString(expression, {
			verbose: true,
			trimHoursLeadingZero: true,
		});
	} catch {
		return "Unable to describe this cron expression";
	}
}

export function formatDuration(durationMs: number): string {
	const ms = Math.max(0, durationMs);
	if (ms < 1_000) return `${ms} ms`;
	if (ms < 60_000) return `${trimDecimal(ms / 1_000)} seconds`;
	if (ms < 3_600_000) return `${trimDecimal(ms / 60_000)} minutes`;
	if (ms < 86_400_000) return `${trimDecimal(ms / 3_600_000)} hours`;
	return `${trimDecimal(ms / 86_400_000)} days`;
}

function trimDecimal(value: number): string {
	return Number.isInteger(value) ? value.toString() : value.toFixed(1);
}
