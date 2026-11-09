import { actor, setup } from "rivetkit";
import { createClient, ActorError } from "rivetkit/client";

const payment = actor({
  actions: { processPayment: async (c, amount: number) => {} }
});

const registry = setup({ use: { payment } });
const client = createClient<typeof registry>("http://localhost:6420");
const paymentActor = client.payment.getOrCreate([]);

try {
  await paymentActor.processPayment(100);
} catch (error) {
  if (error instanceof ActorError) {
    console.log(error.code); // "internal_error"
    console.log(error.message); // "An internal error occurred"

    // Original error details are NOT exposed to the client
    // Check your server logs to see the actual error message
  }
}
