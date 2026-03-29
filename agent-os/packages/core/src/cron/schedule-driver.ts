export interface ScheduleEntry {
	/** Unique ID for this job. */
	id: string;
	/** Standard 5-field cron expression or ISO 8601 timestamp for one-shot. */
	schedule: string;
	/** Called when the schedule fires. */
	callback: () => void | Promise<void>;
}

export interface ScheduleHandle {
	id: string;
}

export interface ScheduleDriver {
	/** Schedule a callback to fire on a cron expression or at a specific time. */
	schedule(entry: ScheduleEntry): ScheduleHandle;

	/** Cancel a previously scheduled entry. */
	cancel(handle: ScheduleHandle): void;

	/** Tear down all scheduled work. */
	dispose(): void;
}
