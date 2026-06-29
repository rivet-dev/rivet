import { actor, setup } from "rivetkit";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

export const primaryServiceActor = actor({
  actions: {
    fetchValue: async () => {
      await new Promise((resolve) => setTimeout(resolve, 500));
      return "primary";
    },
  },
});

export const fallbackServiceActor = actor({
  actions: {
    fetchValue: async () => "fallback",
  },
});

export const timeoutFallbackActor = actor({
  state: {
    lastSource: "none" as "none" | "primary" | "fallback",
    lastValue: "",
  },
  run: workflow(async (ctx) => {
    await ctx.loop("timeout-loop", async (loopCtx) => {
        await loopCtx.queue.nextBatch("wait-request", {
          timeout: 30_000,
        });

        const winner = await loopCtx.race("primary-vs-timeout", [
          {
            name: "primary",
            run: async (raceCtx) =>
              await raceCtx.step("call-primary", async (step) =>
                callPrimaryValue(step),
              ),
          },
          {
            name: "timeout",
            run: async (raceCtx) => {
              await raceCtx.sleep("primary-timeout", 200);
              return "timeout";
            },
          },
        ]);

        let value = winner.value as string;
        let source: "primary" | "fallback" = "primary";

        if (winner.winner === "timeout") {
          value = (await loopCtx.step("fallback-call", async (step) =>
            callFallbackValue(step),
          )) as string;
          source = "fallback";
        }

        await loopCtx.step("record-choice", async (step) => {
          step.state.lastSource = source;
          step.state.lastValue = value;
        });

      });
  }),
  actions: {
    getState: (c) => c.state,
  },
});

async function callPrimaryValue(
  ctx: WorkflowStepContextOf<typeof timeoutFallbackActor>,
): Promise<string> {
  const client = ctx.client();
  const primary = client.primaryServiceActor.getOrCreate(["main"]);
  return await primary.fetchValue();
}

async function callFallbackValue(
  ctx: WorkflowStepContextOf<typeof timeoutFallbackActor>,
): Promise<string> {
  const client = ctx.client();
  const fallback = client.fallbackServiceActor.getOrCreate(["main"]);
  return await fallback.fetchValue();
}

export const registry = setup({
  use: { timeoutFallbackActor, primaryServiceActor, fallbackServiceActor },
});
