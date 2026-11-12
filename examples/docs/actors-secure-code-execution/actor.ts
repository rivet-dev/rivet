import { actor, setup } from "rivetkit";
import { AgentOs } from "@rivet-dev/agentos-core";

export const sandbox = actor({
  // One VM per Actor. It boots with the Actor and is released
  // when the Actor sleeps.
  createVars: async () => ({ vm: await AgentOs.create() }),
  onSleep: async (c) => {
    await c.vars.vm.dispose();
  },
  onDestroy: async (c) => {
    await c.vars.vm.dispose();
  },

  actions: {
    // `code` came from a model. It runs in the VM,
    // not in your process.
    run: async (c, code: string) => {
      const result = await c.vars.vm.javascript.evaluate(code);
      if (result.outcome !== "succeeded") {
        throw new Error(result.error.message);
      }
      return result.value;
    },
  },
});

export const registry = setup({
  use: { sandbox },
});
