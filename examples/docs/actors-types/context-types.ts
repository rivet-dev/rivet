import { actor } from "rivetkit";

const counter = actor({
  // CreateContext in createState hook
  createState: (c, input: { initial: number }): { count: number } => {
    return { count: input.initial };
  },

  // ActionContext in actions
  actions: {
    increment: (c) => {
      c.state.count += 1;
      return c.state.count;
    }
  }
});
