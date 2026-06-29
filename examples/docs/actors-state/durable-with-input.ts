import { actor } from "rivetkit";

interface CounterState {
  count: number;
}

const counter = actor({
  // Compute the initial state from input passed at creation
  createState: (c, input: { startingCount: number }): CounterState => ({
    count: input.startingCount,
  }),

  actions: {
    get: (c) => c.state.count,

    increment: (c) => {
      c.state.count += 1;
      return c.state.count;
    }
  }
});
