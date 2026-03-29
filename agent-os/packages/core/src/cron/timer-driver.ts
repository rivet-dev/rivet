import { Cron } from "croner";
import {
	clearTimeout as clearLongTimeout,
	setTimeout as longSetTimeout,
} from "long-timeout";
import type { LongTimeout } from "long-timeout";
import type { ScheduleDriver, ScheduleEntry, ScheduleHandle } from "./schedule-driver.js";

/**
 * Checks whether a schedule string is a cron expression (as opposed to an
 * ISO 8601 timestamp). Uses a simple heuristic: cron expressions contain
 * spaces and don't contain 'T' or 'Z' characters that are typical of
 * ISO timestamps.
 */
function isCronExpression(schedule: string): boolean {
	return schedule.includes(" ") && !schedule.includes("T") && !schedule.includes("Z");
}

/**
 * Default ScheduleDriver that uses in-process timers. For cron expressions
 * it parses via croner and sets a single timeout for the next fire time,
 * rescheduling after each fire. For ISO 8601 one-shot timestamps it fires
 * once and removes the entry.
 *
 * Uses long-timeout to support delays exceeding setTimeout's 2^31ms limit.
 */
export class TimerScheduleDriver implements ScheduleDriver {
	private timers = new Map<string, LongTimeout>();
	private entries = new Map<string, ScheduleEntry>();

	schedule(entry: ScheduleEntry): ScheduleHandle {
		this.entries.set(entry.id, entry);
		this.scheduleNext(entry);
		return { id: entry.id };
	}

	cancel(handle: ScheduleHandle): void {
		const timer = this.timers.get(handle.id);
		if (timer) {
			clearLongTimeout(timer);
			this.timers.delete(handle.id);
		}
		this.entries.delete(handle.id);
	}

	dispose(): void {
		for (const timer of this.timers.values()) {
			clearLongTimeout(timer);
		}
		this.timers.clear();
		this.entries.clear();
	}

	private scheduleNext(entry: ScheduleEntry): void {
		const isCron = isCronExpression(entry.schedule);
		let next: Date | null;

		if (isCron) {
			const cron = new Cron(entry.schedule);
			next = cron.nextRun();
		} else {
			next = new Date(entry.schedule);
		}

		if (!next) {
			this.entries.delete(entry.id);
			return;
		}

		const delay = Math.max(0, next.getTime() - Date.now());

		const timer = longSetTimeout(async () => {
			this.timers.delete(entry.id);
			try {
				await entry.callback();
			} catch {
				// The driver is fire-and-forget; error handling is the caller's responsibility.
			}
			if (isCron && this.entries.has(entry.id)) {
				this.scheduleNext(entry);
			} else {
				this.entries.delete(entry.id);
			}
		}, delay);

		this.timers.set(entry.id, timer);
	}
}
