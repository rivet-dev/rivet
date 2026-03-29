import type { CreateSessionOptions } from "../agent-os.js";
import type { AgentType } from "../agents.js";

export type CronAction =
	| {
			type: "session";
			agentType: AgentType;
			prompt: string;
			options?: CreateSessionOptions;
	  }
	| { type: "exec"; command: string; args?: string[] }
	| { type: "callback"; fn: () => void | Promise<void> };

export interface CronJobOptions {
	/** Optional ID. Auto-generated UUID if omitted. */
	id?: string;
	/** Standard 5-field cron expression or ISO 8601 timestamp for one-shot. */
	schedule: string;
	/** What to do when the schedule fires. */
	action: CronAction;
	/** What to do if previous execution is still running. Default: 'allow'. */
	overlap?: "allow" | "skip" | "queue";
}

export interface CronJob {
	id: string;
	cancel(): void;
}

export interface CronJobInfo {
	id: string;
	schedule: string;
	action: CronAction;
	overlap: "allow" | "skip" | "queue";
	lastRun?: Date;
	nextRun?: Date;
	runCount: number;
	running: boolean;
}

export type CronEvent =
	| { type: "cron:fire"; jobId: string; time: Date }
	| { type: "cron:complete"; jobId: string; time: Date; durationMs: number }
	| { type: "cron:error"; jobId: string; time: Date; error: Error };

export type CronEventHandler = (event: CronEvent) => void;
