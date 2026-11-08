import { actor } from "rivetkit";
import { workflow } from "rivetkit/workflow";

// Example: Finite job that destroys the actor when done
const oneShotJob = actor({
  run: workflow(async (ctx) => {
    // A completed step is recorded, so a restart after the job finishes does not run it again
    await ctx.step("process-job", async () => {
      await processJob();
    });

    await ctx.step("destroy", async (step) => {
      step.destroy();
    });
  }),
});

async function processJob(): Promise<void> {}
