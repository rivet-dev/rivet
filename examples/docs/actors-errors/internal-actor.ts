import { actor } from "rivetkit";

const payment = actor({
  state: { transactions: [] },
  actions: {
    processPayment: async (c, amount: number) => {
      // This will throw a regular Error (not UserError)
      const result = await fetch("https://payment-api.example.com/charge", {
        method: "POST",
        body: JSON.stringify({ amount })
      });

      if (!result.ok) {
        // This internal error will be hidden from the client
        throw new Error(`Payment API returned ${result.status}: ${await result.text()}`);
      }

      // Rest of payment logic...
    }
  }
});
