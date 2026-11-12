import { actor } from "rivetkit";
import { AgentOs } from "@rivet-dev/agentos-core";

export const evaluator = actor({
  actions: {
    // Boot a fresh VM for this call and release it at the end.
    evaluate: async (c, code: string) => {
      const vm = await AgentOs.create();
      try {
        const result = await vm.javascript.evaluate(code);
        if (result.outcome !== "succeeded") {
          throw new Error(result.error.message);
        }
        return result.value;
      } finally {
        await vm.dispose();
      }
    },
  },
});
