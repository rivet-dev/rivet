import { actor } from "rivetkit";

const checkout = actor({
  state: { status: "pending" as "pending" | "charged" | "fulfilled" },

  actions: {
    fulfill: async (c) => {
      c.state.status = "charged";
      // Persist before the side effect so a crash can't undo it
      await c.saveState({ immediate: true });

      await chargeExternalProvider();

      c.state.status = "fulfilled";
      return c.state.status;
    }
  }
});

async function chargeExternalProvider() {
  await new Promise((resolve) => setTimeout(resolve, 100));
}
