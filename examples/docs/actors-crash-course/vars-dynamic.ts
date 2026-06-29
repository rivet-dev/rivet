import { actor } from "rivetkit";

const counter = actor({
  state: { count: 0 },
  createVars: () => ({
    emitter: new EventTarget(),
  }),
  actions: {
    increment: (c) => {
      c.vars.emitter.dispatchEvent(new Event("change"));
      return c.state.count += 1;
    },
  },
});
