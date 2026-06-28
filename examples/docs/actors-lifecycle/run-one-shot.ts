import { actor } from "rivetkit";

// Example: Finite task that destroys the actor when done
const oneShotJob = actor({
  run: async (c) => {
    await processJob();
    c.destroy();
  },
});

async function processJob(): Promise<void> {}
