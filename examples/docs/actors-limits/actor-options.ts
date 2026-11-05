import { actor } from "rivetkit";

const myActor = actor({
  options: {
    maxQueueSize: 1000,
    actionTimeout: 60_000,
    sleepTimeout: 30_000,
  },
  // ...
});
