/**
 * Overview: Cron scheduling actions for the agent-os HOC. Serializes
 * 0.2.8 `CronActionInfo` (callback closures stripped) for the actor wire.
 */
import type {
	CronAction,
	CronActionInfo,
	CronJobInfo,
} from "@rivet-dev/agent-os-core";
import type { AgentOsActorConfig } from "../config";
import type {
	AgentOsActionContext,
	SerializableCronAction,
	SerializableCronJobInfo,
	SerializableCronJobOptions,
} from "../types";
import { ensureVm } from "./index";

// listCronJobs returns CronActionInfo (callback is kind-only, no fn).
function serializeCronAction(action: CronActionInfo): SerializableCronAction {
	switch (action.type) {
		case "session":
			return {
				type: "session",
				agentType: action.agentType,
				prompt: action.prompt,
				cwd: action.options?.cwd,
			};
		case "exec":
			return {
				type: "exec",
				command: action.command,
				args: action.args,
			};
		case "callback":
			throw new TypeError("callback cron actions are not serializable");
	}
}

function serializeCronJob(job: CronJobInfo): SerializableCronJobInfo {
	// 0.2.8 already exposes lastRun/nextRun as ISO strings.
	return {
		id: job.id,
		schedule: job.schedule,
		action: serializeCronAction(job.action),
		overlap: job.overlap,
		lastRun: job.lastRun,
		nextRun: job.nextRun,
		runCount: job.runCount,
		running: job.running,
	};
}

// Build cron scheduling actions for the actor factory.
export function buildCronActions<TConnParams>(
	config: AgentOsActorConfig<TConnParams>,
) {
	return {
		scheduleCron: async (
			c: AgentOsActionContext<TConnParams>,
			options: SerializableCronJobOptions,
		): Promise<{ id: string }> => {
			const agentOs = await ensureVm(c, config);
			const job = agentOs.scheduleCron({
				id: options.id,
				schedule: options.schedule,
				action: options.action as CronAction,
				overlap: options.overlap,
			});
			c.log.info({
				msg: "agent-os cron job scheduled",
				jobId: job.id,
				schedule: options.schedule,
			});
			return { id: job.id };
		},

		listCronJobs: async (
			c: AgentOsActionContext<TConnParams>,
		): Promise<SerializableCronJobInfo[]> => {
			const agentOs = await ensureVm(c, config);
			return agentOs.listCronJobs().map(serializeCronJob);
		},

		cancelCronJob: async (
			c: AgentOsActionContext<TConnParams>,
			id: string,
		): Promise<void> => {
			const agentOs = await ensureVm(c, config);
			agentOs.cancelCronJob(id);
			c.log.info({ msg: "agent-os cron job cancelled", jobId: id });
		},
	};
}
