import { actor } from "rivetkit";

const sessionActor = actor({
  vars: {
    activeTurns: 0,
  },

  actions: {
    runTurn: async (c, input: string) => {
      c.vars.activeTurns += 1;
      try {
        const result = await c.keepAwake(processTurn(input));
        return result;
      } finally {
        c.vars.activeTurns -= 1;
      }
    },
  }
});

declare function processTurn(input: string): Promise<string>;
