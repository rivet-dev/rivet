import { actor } from "rivetkit";

const sessionActor = actor({
  state: {
    activeTurns: 0,
  },

  actions: {
    runTurn: async (c, input: string) => {
      c.state.activeTurns += 1;
      try {
        const result = await c.keepAwake(processTurn(input));
        return result;
      } finally {
        c.state.activeTurns -= 1;
      }
    },
  }
});

declare function processTurn(input: string): Promise<string>;
