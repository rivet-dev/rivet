import { actor } from "rivetkit";
import { workflow } from "rivetkit/workflow";

const orderProcessor = actor({
  options: {
    name: "Order Processor", // Shown instead of the registry key
    icon: "cart-shopping",   // Overrides the workflow icon
  },
  run: workflow(async (ctx) => {
    await ctx.step("process-order", async () => {
      // ...
    });
  }),
});
