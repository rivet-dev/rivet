import { actor } from "rivetkit";

const counter = actor({
  state: { count: 0 },

  onCreate: (c, input: { initialCount: number }) => {
    console.log("Actor created with initial count:", input.initialCount);
  },

  actions: { /* ... */ }
});
