import { actor, ActionContextOf } from "rivetkit";

const counter = actor({
  state: { count: 0 },

  actions: {
    increment: (c) => {
      incrementCount(c);
    }
  }
});

// Simple helper function with typed context
function incrementCount(c: ActionContextOf<typeof counter>) {
  c.state.count += 1;
}
