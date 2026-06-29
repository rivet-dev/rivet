import { actor } from "rivetkit";

const counter = actor({
  state: { count: 0 },

  onMigrate: (c, isNew) => {
    // Run database migrations before any other lifecycle hook
  },

  actions: { /* ... */ }
});
