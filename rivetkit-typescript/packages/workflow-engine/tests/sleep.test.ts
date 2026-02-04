import { beforeEach, describe, expect, it } from "vitest";
import {
	InMemoryDriver,
	runWorkflow,
	type WorkflowContextInterface,
} from "../src/testing.js";

const modes = ["yield", "live"] as const;

for (const mode of modes) {
	describe(`Workflow Engine Sleep (${mode})`, { sequential: true }, () => {
		let driver: InMemoryDriver;

		beforeEach(() => {
			driver = new InMemoryDriver();
			driver.latency = 0;
		});

		it("should yield on long sleep", async () => {
			const durationMs = mode === "live" ? 150 : 10000;
			const workflow = async (ctx: WorkflowContextInterface) => {
				await ctx.sleep("my-sleep", durationMs);
				return "done";
			};

			const result = await runWorkflow(
				"wf-1",
				workflow,
				undefined,
				driver,
				{ mode },
			).result;

			if (mode === "yield") {
				expect(result.state).toBe("sleeping");
				expect(result.sleepUntil).toBeDefined();
				expect(result.sleepUntil).toBeGreaterThan(Date.now());
				return;
			}

			expect(result.state).toBe("completed");
			expect(result.output).toBe("done");
		});

		it("should complete short sleep in memory", async () => {
			driver.workerPollInterval = 1000;

			const workflow = async (ctx: WorkflowContextInterface) => {
				await ctx.sleep("short-sleep", 10);
				return "done";
			};

			const result = await runWorkflow(
				"wf-1",
				workflow,
				undefined,
				driver,
				{
					mode,
				},
			).result;

			expect(result.state).toBe("completed");
			expect(result.output).toBe("done");
			expect(driver.getAlarm("wf-1")).toBeUndefined();
		});

		it("should resume after sleep deadline", async () => {
			driver.workerPollInterval = 0;

			const workflow = async (ctx: WorkflowContextInterface) => {
				await ctx.sleep("my-sleep", 20);
				return "done";
			};

			if (mode === "yield") {
				const result1 = await runWorkflow(
					"wf-1",
					workflow,
					undefined,
					driver,
					{ mode },
				).result;
				expect(result1.state).toBe("sleeping");

				await new Promise((r) => setTimeout(r, 30));

				const result2 = await runWorkflow(
					"wf-1",
					workflow,
					undefined,
					driver,
					{ mode },
				).result;

				expect(result2.state).toBe("completed");
				expect(result2.output).toBe("done");
				return;
			}

			const result = await runWorkflow(
				"wf-1",
				workflow,
				undefined,
				driver,
				{ mode },
			).result;

			expect(result.state).toBe("completed");
			expect(result.output).toBe("done");
		});

		it("should complete sleepUntil with past timestamp", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				await ctx.sleepUntil("past", Date.now() - 1);
				return "done";
			};

			const result = await runWorkflow(
				"wf-1",
				workflow,
				undefined,
				driver,
				{
					mode,
				},
			).result;

			expect(result.state).toBe("completed");
			expect(result.output).toBe("done");
		});

		it("should keep short sleeps in memory near poll interval", async () => {
			driver.workerPollInterval = 50;

			const workflow = async (ctx: WorkflowContextInterface) => {
				await ctx.sleep("near-poll", 25);
				return "done";
			};

			const result = await runWorkflow(
				"wf-1",
				workflow,
				undefined,
				driver,
				{
					mode,
				},
			).result;

			expect(result.state).toBe("completed");
			expect(driver.getAlarm("wf-1")).toBeUndefined();
		});

		it("should schedule and clear alarms for long sleep", async () => {
			driver.workerPollInterval = 1;
			const durationMs = mode === "live" ? 200 : 20;

			const workflow = async (ctx: WorkflowContextInterface) => {
				await ctx.sleep("alarm-sleep", durationMs);
				return "done";
			};

			const handle = runWorkflow("wf-1", workflow, undefined, driver, {
				mode,
			});

			if (mode === "yield") {
				const result1 = await handle.result;
				expect(result1.state).toBe("sleeping");
				expect(driver.getAlarm("wf-1")).toBe(result1.sleepUntil);

				await new Promise((r) => setTimeout(r, 30));

				const result2 = await runWorkflow(
					"wf-1",
					workflow,
					undefined,
					driver,
					{ mode },
				).result;
				expect(result2.state).toBe("completed");
				expect(driver.getAlarm("wf-1")).toBeUndefined();
				return;
			}

			const alarm = await new Promise<number | undefined>((resolve) => {
				const start = Date.now();
				const check = () => {
					const value = driver.getAlarm("wf-1");
					if (value !== undefined || Date.now() - start > 50) {
						resolve(value);
						return;
					}
					setTimeout(check, 1);
				};
				check();
			});
			expect(alarm).toBeDefined();

			const result = await handle.result;
			expect(result.state).toBe("completed");
			expect(driver.getAlarm("wf-1")).toBeUndefined();
		});
	});
}
