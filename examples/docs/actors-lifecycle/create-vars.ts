import { actor } from "rivetkit";

interface CounterVars {
  lastAccessTime: number;
  emitter: EventTarget;
}

const counter = actor({
  createVars: (c): CounterVars => ({
    lastAccessTime: Date.now(),
    emitter: new EventTarget()
  }),
  actions: { /* ... */ }
});
