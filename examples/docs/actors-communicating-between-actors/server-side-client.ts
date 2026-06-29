import { actor, setup } from "rivetkit";

interface Order {
  id: string;
  customerId: string;
  quantity: number;
  amount: number;
}

interface ProcessedOrder extends Order {
  status: string;
  paymentResult: { transactionId: string };
}

const inventory = actor({
  state: { stock: 100 },
  actions: {
    reserveStock: (c, quantity: number) => {
      c.state.stock -= quantity;
      return { reserved: quantity };
    }
  }
});

const payment = actor({
  state: {},
  actions: {
    processPayment: (c, amount: number) => ({ transactionId: "tx-123" })
  }
});

const orderProcessor = actor({
  state: { orders: [] as ProcessedOrder[] },

  actions: {
    processOrder: async (c, order: Order) => {
      const client = c.client<typeof registry>();

      // Reserve the stock
      const inventoryHandle = client.inventory.getOrCreate(["main"]);
      await inventoryHandle.reserveStock(order.quantity);

      // Process payment through payment actor
      const paymentHandle = client.payment.getOrCreate([order.customerId]);
      const result = await paymentHandle.processPayment(order.amount);

      // Update order state
      c.state.orders.push({ ...order, status: "completed", paymentResult: result });

      return { success: true, orderId: order.id };
    }
  }
});

const registry = setup({ use: { inventory, payment, orderProcessor } });
