import { actor, event, setup } from "rivetkit";
import { workflow, type WorkflowErrorEvent } from "rivetkit/workflow";

export const errorHookActor = actor({
  state: {
    lastError: null as WorkflowErrorEvent | null,
  },
  events: {
    workflowError: event<[WorkflowErrorEvent]>(),
  },
  run: workflow(
    async (ctx) => {
      await ctx.step({
        name: "sync-ledger",
        maxRetries: 3,
        retryBackoffBase: 250,
        retryBackoffMax: 1_000,
        run: async (ctx) => {
          throw new Error("ledger unavailable");
        },
      });
    },
    {
      onError: (c, event) => {
        c.state.lastError = event;
        c.broadcast("workflowError", event);
      },
    },
  ),
  actions: {
    getState: (c) => c.state,
  },
});

export const registry = setup({ use: { errorHookActor } });
