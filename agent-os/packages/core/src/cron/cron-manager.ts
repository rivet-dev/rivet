import { randomUUID } from "node:crypto";
import { Cron } from "croner";
import type { AgentOs, CreateSessionOptions } from "../agent-os.js";
import type { AgentType } from "../agents.js";
import type { ScheduleDriver, ScheduleHandle } from "./schedule-driver.js";
import type {
	CronAction,
	CronEvent,
	CronEventHandler,
	CronJob,
	CronJobInfo,
	CronJobOptions,
} from "./types.js";

interface CronJobState {
	id: string;
	schedule: string;
	action: CronAction;
	overlap: "allow" | "skip" | "queue";
	handle: ScheduleHandle;
	lastRun?: Date;
	nextRun?: Date;
	runCount: number;
	running: boolean;
	queued: boolean;
}

/**
 * Compute the next fire time for a schedule string. Returns undefined if
 * the schedule is a one-shot ISO timestamp in the past or if croner
 * cannot determine a next run.
 */
function computeNextTime(schedule: string): Date | undefined {
	if (schedule.includes(" ") && !schedule.includes("T") && !schedule.includes("Z")) {
		const cron = new Cron(schedule);
		return cron.nextRun() ?? undefined;
	}
	const date = new Date(schedule);
	return date.getTime() > Date.now() ? date : undefined;
}

/**
 * Internal class that bridges ScheduleDriver and AgentOs. Owns the job
 * registry, executes actions, and emits lifecycle events.
 */
export class CronManager {
	private jobs = new Map<string, CronJobState>();
	private driver: ScheduleDriver;
	private vm: AgentOs;
	private listeners: CronEventHandler[] = [];

	constructor(vm: AgentOs, driver: ScheduleDriver) {
		this.vm = vm;
		this.driver = driver;
	}

	schedule(options: CronJobOptions): CronJob {
		const id = options.id ?? randomUUID();
		const overlap = options.overlap ?? "allow";

		const handle = this.driver.schedule({
			id,
			schedule: options.schedule,
			callback: () => this.executeJob(id),
		});

		const state: CronJobState = {
			id,
			schedule: options.schedule,
			action: options.action,
			overlap,
			handle,
			lastRun: undefined,
			nextRun: computeNextTime(options.schedule),
			runCount: 0,
			running: false,
			queued: false,
		};

		this.jobs.set(id, state);
		return { id, cancel: () => this.cancel(id) };
	}

	cancel(id: string): void {
		const state = this.jobs.get(id);
		if (!state) return;
		this.driver.cancel(state.handle);
		this.jobs.delete(id);
	}

	list(): CronJobInfo[] {
		const result: CronJobInfo[] = [];
		for (const state of this.jobs.values()) {
			result.push({
				id: state.id,
				schedule: state.schedule,
				action: state.action,
				overlap: state.overlap,
				lastRun: state.lastRun,
				nextRun: state.nextRun,
				runCount: state.runCount,
				running: state.running,
			});
		}
		return result;
	}

	onEvent(handler: CronEventHandler): void {
		this.listeners.push(handler);
	}

	dispose(): void {
		for (const state of this.jobs.values()) {
			this.driver.cancel(state.handle);
		}
		this.jobs.clear();
		this.driver.dispose();
	}

	private emit(event: CronEvent): void {
		for (const handler of this.listeners) {
			try {
				handler(event);
			} catch {
				// Event handler errors must not crash the manager.
			}
		}
	}

	private async executeJob(id: string): Promise<void> {
		const state = this.jobs.get(id);
		if (!state) return;

		// Overlap policy.
		if (state.running && state.overlap === "skip") {
			return;
		}
		if (state.running && state.overlap === "queue") {
			state.queued = true;
			return;
		}

		state.running = true;
		state.lastRun = new Date();
		state.runCount++;

		this.emit({ type: "cron:fire", jobId: state.id, time: new Date() });

		const startTime = Date.now();
		try {
			await this.runAction(state.action);
			this.emit({
				type: "cron:complete",
				jobId: state.id,
				time: new Date(),
				durationMs: Date.now() - startTime,
			});
		} catch (error) {
			this.emit({
				type: "cron:error",
				jobId: state.id,
				time: new Date(),
				error: error as Error,
			});
		} finally {
			state.running = false;
			state.nextRun = computeNextTime(state.schedule);

			// Process queued execution.
			if (state.queued) {
				state.queued = false;
				void this.executeJob(id);
			}
		}
	}

	private async runAction(action: CronAction): Promise<void> {
		switch (action.type) {
			case "session": {
				const { sessionId } = await this.vm.createSession(
					action.agentType,
					action.options,
				);
				try {
					await this.vm.prompt(sessionId, action.prompt);
				} finally {
					this.vm.closeSession(sessionId);
				}
				break;
			}
			case "exec": {
				const cmd = action.args?.length
					? `${action.command} ${action.args.join(" ")}`
					: action.command;
				await this.vm.exec(cmd);
				break;
			}
			case "callback": {
				await action.fn();
				break;
			}
		}
	}
}
