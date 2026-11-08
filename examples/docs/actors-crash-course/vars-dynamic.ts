import { actor } from "rivetkit";

const counter = actor({
  createVars: () => ({
    count: 0,
    emitter: new EventTarget(),
  }),
  actions: {
    increment: (c) => {
      c.vars.emitter.dispatchEvent(new Event("change"));
      return c.vars.count += 1;
    },
  },
});
