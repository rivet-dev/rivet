import { actor } from "rivetkit";

const counter = actor({
  createState: (c, input: { initialCount: number }) => ({
    count: input.initialCount
  }),
  actions: { /* ... */ }
});
