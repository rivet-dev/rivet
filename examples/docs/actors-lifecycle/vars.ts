import { actor } from "rivetkit";

const counter = actor({
  state: { count: 0 },
  vars: { lastAccessTime: 0 },
  actions: { /* ... */ }
});
