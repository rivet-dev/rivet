import { actor } from "rivetkit";

const gameSession = actor({
  onDestroy: (c) => {
    // Clean up any external resources
  },
  actions: { /* ... */ }
});
