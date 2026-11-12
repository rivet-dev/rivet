import { actor } from "rivetkit";

// An actor that still uses a plain `run` function
const poller = actor({
  createVars: () => ({ lastPrice: 0 }),
  run: async (c) => {
    while (!c.aborted) {
      c.vars.lastPrice = await fetchPrice();
      await new Promise((resolve) => setTimeout(resolve, 5_000));
    }
  },
  actions: {
    price: (c) => c.vars.lastPrice,
  },
});

async function fetchPrice(): Promise<number> {
  return 42;
}
