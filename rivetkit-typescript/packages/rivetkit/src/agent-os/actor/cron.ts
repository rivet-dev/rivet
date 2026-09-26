import type {
	AgentType,
	CronAction,
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

function serializeCronAction(action: CronAction): SerializableCronAction {
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

function deserializeCronAction(action: SerializableCronAction): CronAction {
	switch (action.type) {
		case "session":
			return {
				type: "session",
				agentType: action.agentType as AgentType,
				prompt: action.prompt,
				options:
					action.cwd !== undefined ? { cwd: action.cwd } : undefined,
			};
		case "exec":
			return {
				type: "exec",
				command: action.command,
				args: action.args,
			};
	}
}

function serializeCronJob(job: CronJobInfo): SerializableCronJobInfo {
	return {
		id: job.id,
		schedule: job.schedule,
		action: serializeCronAction(job.action),
		overlap: job.overlap,
		lastRun: job.lastRun?.toISOString(),
		nextRun: job.nextRun?.toISOString(),
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
				action: deserializeCronAction(options.action),
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
