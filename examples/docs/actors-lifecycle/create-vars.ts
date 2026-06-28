import { actor } from "rivetkit";

interface CounterVars {
  lastAccessTime: number;
  emitter: EventTarget;
}

const counter = actor({
  state: { count: 0 },
  createVars: (c): CounterVars => ({
    lastAccessTime: Date.now(),
    emitter: new EventTarget()
  }),
  actions: { /* ... */ }
});
