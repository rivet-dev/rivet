import { actor, setup } from "rivetkit";

const counter = actor({
  state: { value: 0 },
  actions: {
    // Use an action when the caller needs a response.
    increment: (c, amount: number) => {
      c.state.value += amount;
      return { value: c.state.value };
    },
  },
});

export const registry = setup({ use: { counter } });
