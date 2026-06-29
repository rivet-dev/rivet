import { actor, setup } from "rivetkit";
import { createClient, ActorError } from "rivetkit/client";

const payment = actor({
  state: {},
  actions: { processPayment: async (c, amount: number) => {} }
});

const registry = setup({ use: { payment } });
const client = createClient<typeof registry>("http://localhost:6420");
const paymentActor = client.payment.getOrCreate([]);

// With RIVET_EXPOSE_ERRORS=1
try {
  await paymentActor.processPayment(100);
} catch (error) {
  if (error instanceof ActorError) {
    console.log(error.message);
    // "Payment API returned 402: Insufficient funds"
    // Instead of: "An internal error occurred"
  }
}
