import { actor } from "rivetkit";

interface CounterState {
  count: number;
}

const counter = actor({
  createState: (c, input: { start?: number }): CounterState => ({
    count: input.start ?? 0,
  }),
  actions: {
    increment: (c) => c.state.count += 1,
  },
});
