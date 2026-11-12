import { actor } from "rivetkit";

const counter = actor({
  onMigrate: (c, isNew) => {
    // Run database migrations before any other lifecycle hook
  },

  actions: { /* ... */ }
});
