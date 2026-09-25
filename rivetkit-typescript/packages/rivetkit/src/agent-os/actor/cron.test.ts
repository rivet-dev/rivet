import type {
	AgentOs,
	CronJobInfo,
	CronJobOptions,
} from "@rivet-dev/agent-os-core";
import { describe, expect, test } from "vitest";
import type { AgentOsActorConfig } from "../config";
import type { AgentOsActionContext } from "../types";
import { buildCronActions } from "./cron";

// Records the options the actor passes to agent-os-core and reports them back
// through listCronJobs the same way the core cron manager does.
function createContext() {
	const scheduled: CronJobOptions[] = [];
	const agentOs = {
		scheduleCron(options: CronJobOptions) {
			scheduled.push(options);
			return { id: options.id ?? "job-1" };
		},
		listCronJobs(): CronJobInfo[] {
			return scheduled.map((options) => ({
				id: options.id ?? "job-1",
				schedule: options.schedule,
				action: options.action,
				overlap: options.overlap ?? "allow",
				runCount: 0,
				running: false,
			}));
		},
	} as unknown as AgentOs;
	const c = {
		vars: { agentOs },
		log: { info: () => {} },
	} as unknown as AgentOsActionContext<undefined>;
	return { c, scheduled };
}

describe("agent-os cron actions", () => {
	test("scheduleCron passes the session cwd to agent-os-core", async () => {
		const { c, scheduled } = createContext();
		const actions = buildCronActions({} as AgentOsActorConfig<undefined>);

		await actions.scheduleCron(c, {
			id: "nightly",
			schedule: "0 0 * * *",
			action: {
				type: "session",
				agentType: "pi",
				prompt: "summarize the repo",
				cwd: "/home/user/project",
			},
		});

		expect(scheduled[0]?.action).toEqual({
			type: "session",
			agentType: "pi",
			prompt: "summarize the repo",
			options: { cwd: "/home/user/project" },
		});
		expect(await actions.listCronJobs(c)).toMatchObject([
			{
				id: "nightly",
				action: {
					type: "session",
					agentType: "pi",
					prompt: "summarize the repo",
					cwd: "/home/user/project",
				},
			},
		]);
	});

	test("scheduleCron passes exec actions through unchanged", async () => {
		const { c, scheduled } = createContext();
		const actions = buildCronActions({} as AgentOsActorConfig<undefined>);

		await actions.scheduleCron(c, {
			schedule: "* * * * *",
			action: { type: "exec", command: "echo", args: ["tick"] },
		});

		expect(scheduled[0]?.action).toEqual({
			type: "exec",
			command: "echo",
			args: ["tick"],
		});
	});
});
