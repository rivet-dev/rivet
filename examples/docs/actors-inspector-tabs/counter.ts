import { actor, setup } from "rivetkit";

export const counter = actor({
  state: { value: 0 },
  actions: {
    increment: (c, amount: number) => {
      c.state.value += amount;
      return c.state.value;
    },
  },
  inspector: {
    tabs: [
      {
        id: "counter",
        label: "Counter",
        icon: "tag",
        source: "./inspector-tabs/counter",
      },
      { id: "queue", hidden: true },
    ],
  },
});

export const registry = setup({ use: { counter } });
registry.start();
