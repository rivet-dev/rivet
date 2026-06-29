import { actor } from "rivetkit";

interface CounterState {
  count: number;
}

const counter = actor({
  // Compute the initial state when the actor is created
  createState: (): CounterState => ({ count: 0 }),

  actions: {
    get: (c) => c.state.count,

    increment: (c) => {
      c.state.count += 1;
      return c.state.count;
    }
  }
});
