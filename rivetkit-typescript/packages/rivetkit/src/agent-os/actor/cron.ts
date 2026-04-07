import type { CronAction, CronJobInfo } from "@rivet-dev/agent-os-core";
import type { AgentOsActorConfig } from "../config";
import type {
	AgentOsActionContext,
	SerializableCronJobOptions,
} from "../types";
import { ensureVm } from "./index";

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
		): Promise<CronJobInfo[]> => {
			const agentOs = await ensureVm(c, config);
			return agentOs.listCronJobs();
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
