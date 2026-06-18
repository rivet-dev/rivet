import { beforeEach, describe, expect, it } from "vitest";
import {
	HistoryDivergedError,
	InMemoryDriver,
	Loop,
	runWorkflow,
	type WorkflowContextInterface,
} from "../src/testing.js";

const modes = ["yield", "live"] as const;

for (const mode of modes) {
	describe(
		`Workflow Engine getVersion (${mode})`,
		{ sequential: true },
		() => {
			let driver: InMemoryDriver;

			beforeEach(() => {
				driver = new InMemoryDriver();
				driver.latency = 0;
			});

			it("resolves a fresh instance to latest and pins it on replay", async () => {
				const workflow = async (ctx: WorkflowContextInterface) => {
					const v = await ctx.getVersion("gate", 2);
					await ctx.step("record", async () => v);
					return v;
				};

				const first = await runWorkflow(
					"wf-fresh",
					workflow,
					undefined,
					driver,
					{ mode },
				).result;
				expect(first.state).toBe("completed");
				expect(first.output).toBe(2);

				// Re-running replays the recorded version_check and returns the
				// same pinned value.
				const second = await runWorkflow(
					"wf-fresh",
					workflow,
					undefined,
					driver,
					{ mode },
				).result;
				expect(second.state).toBe("completed");
				expect(second.output).toBe(2);
			});

			it("resolves an old in-flight instance to floor 1 and replays the old step's value", async () => {
				// v1 code: records a step, no version gate.
				const v1 = async (ctx: WorkflowContextInterface) => {
					await ctx.step("work", async () => "did-work");
					return "v1-done";
				};
				await runWorkflow("wf-old", v1, undefined, driver, { mode })
					.result;

				// v2 code: adds a gate before the step. The old instance already
				// ran `work`, so the gate must resolve to 1 (old branch) and the
				// replayed step must return the originally recorded value.
				const v2 = async (ctx: WorkflowContextInterface) => {
					const v = await ctx.getVersion("gate", 2);
					if (v === 1) {
						const w = await ctx.step(
							"work",
							async () => "should-not-run",
						);
						return `old:${w}`;
					}
					return "new";
				};
				const result = await runWorkflow(
					"wf-old",
					v2,
					undefined,
					driver,
					{ mode },
				).result;
				expect(result.state).toBe("completed");
				expect(result.output).toBe("old:did-work");
			});

			it("resolves to latest when the gate is not the first call in its scope (fresh run)", async () => {
				const workflow = async (ctx: WorkflowContextInterface) => {
					await ctx.step("a", async () => "a");
					return await ctx.getVersion("gate", 3);
				};
				const result = await runWorkflow(
					"wf-notfirst",
					workflow,
					undefined,
					driver,
					{ mode },
				).result;
				expect(result.state).toBe("completed");
				expect(result.output).toBe(3);
			});

			it("can be retired with removed() once a migration is finished", async () => {
				const withGate = async (ctx: WorkflowContextInterface) => {
					const v = await ctx.getVersion("gate", 2);
					return `v${v}`;
				};
				const gated = await runWorkflow(
					"wf-retire",
					withGate,
					undefined,
					driver,
					{ mode },
				).result;
				expect(gated.output).toBe("v2");

				const removeGate = async (ctx: WorkflowContextInterface) => {
					await ctx.removed("gate", "version_check");
					return "cleaned";
				};
				const result = await runWorkflow(
					"wf-retire",
					removeGate,
					undefined,
					driver,
					{ mode },
				).result;
				expect(result.state).toBe("completed");
				expect(result.output).toBe("cleaned");
			});

			it("throws HistoryDiverged when a non-version entry occupies the gate location", async () => {
				const asStep = async (ctx: WorkflowContextInterface) => {
					await ctx.step("gate", async () => "x");
					return "step";
				};
				await runWorkflow("wf-diverge", asStep, undefined, driver, {
					mode,
				}).result;

				const asVersion = async (ctx: WorkflowContextInterface) => {
					await ctx.getVersion("gate", 2);
					return "version";
				};
				await expect(
					runWorkflow("wf-diverge", asVersion, undefined, driver, {
						mode,
					}).result,
				).rejects.toThrow(HistoryDivergedError);
			});
		},
	);
}

describe("Workflow Engine getVersion per-iteration cutover", { sequential: true }, () => {
	it("resolves each loop iteration independently across a redeploy", async () => {
		const driver = new InMemoryDriver();
		driver.latency = 0;
		const mode = "yield" as const;

		// v1 loop: records a `pre` step before waiting for a message, so an
		// iteration can be left suspended mid-body with history already present.
		const v1 = async (ctx: WorkflowContextInterface) => {
			return await ctx.loop({
				name: "consume",
				state: { i: 0, processed: 0 },
				run: async (lctx, state) => {
					await lctx.step("pre", async () => `pre-${state.i}`);
					const msg = await lctx.queue.next<string>("in", {
						names: ["work"],
					});
					await lctx.step("post", async () => `post:${msg.body}`);
					const processed = state.processed + 1;
					if (processed >= 3) {
						return Loop.break(processed);
					}
					return Loop.continue({ i: state.i + 1, processed });
				},
			});
		};

		// First message is processed in iteration 0; iteration 1 records `pre`
		// then suspends at queue.next (in-flight under v1).
		await driver.messageDriver.addMessage({
			id: "m1",
			name: "work",
			data: "one",
			sentAt: Date.now(),
		});
		const r1 = await runWorkflow("wf-loop", v1, undefined, driver, { mode })
			.result;
		expect(r1.state).toBe("sleeping");

		// Redeploy v2: adds a gate at the top of the loop body.
		const seenByIter = new Map<number, number>();
		const v2 = async (ctx: WorkflowContextInterface) => {
			return await ctx.loop({
				name: "consume",
				state: { i: 0, processed: 0 },
				run: async (lctx, state) => {
					const v = await lctx.getVersion("gate", 2);
					seenByIter.set(state.i, v);
					await lctx.step("pre", async () => `pre-${state.i}`);
					const msg = await lctx.queue.next<string>("in", {
						names: ["work"],
					});
					await lctx.step("post", async () => `post:${msg.body}`);
					const processed = state.processed + 1;
					if (processed >= 3) {
						return Loop.break(processed);
					}
					return Loop.continue({ i: state.i + 1, processed });
				},
			});
		};

		// Second message resumes the in-flight iteration 1 (whose `pre` step
		// predates the gate) and lets iteration 2 begin fresh.
		await driver.messageDriver.addMessage({
			id: "m2",
			name: "work",
			data: "two",
			sentAt: Date.now(),
		});
		const r2 = await runWorkflow("wf-loop", v2, undefined, driver, { mode })
			.result;
		expect(r2.state).toBe("sleeping");

		// Third message completes iteration 2.
		await driver.messageDriver.addMessage({
			id: "m3",
			name: "work",
			data: "three",
			sentAt: Date.now(),
		});
		const r3 = await runWorkflow("wf-loop", v2, undefined, driver, { mode })
			.result;
		expect(r3.state).toBe("completed");

		// Iteration 1 was in-flight under v1 (its `pre` step predates the gate),
		// so it resolves to floor version 1. Iteration 2 is fresh, so it
		// resolves to latest (2).
		expect(seenByIter.get(1)).toBe(1);
		expect(seenByIter.get(2)).toBe(2);
	});
});
