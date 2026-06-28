import { actor, setup } from "rivetkit";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

export const versionedWorkflowActor = actor({
  state: {
    runs: 0,
  },
  run: workflow(async (ctx) => {
    await ctx.step("validate-v2", async (step) => {
      step.state.runs += 1;
    });

    await ctx.removed("validate-v1", "step");

    await ctx.loop("main-loop-v2", async (loopCtx) => {
        await loopCtx.sleep("idle", 500);
        await loopCtx.step("heartbeat-v2", async (step) => {
          step.state.runs += 1;
        });
      });
  }),
  actions: {
    getState: (c) => c.state,
  },
});

export const registry = setup({ use: { versionedWorkflowActor } });
