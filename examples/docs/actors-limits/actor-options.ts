import { actor } from "rivetkit";

const myActor = actor({
  options: {
    actionTimeout: 60_000,
    sleepTimeout: 30_000,
    sleepGracePeriod: 15_000,
  },
  // ...
});
