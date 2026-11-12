import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";

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
  db: db({
    onMigrate: async (db) => {
      await db.execute(
        "CREATE TABLE IF NOT EXISTS stock (id INTEGER PRIMARY KEY CHECK (id = 1), count INTEGER NOT NULL)",
      );
      await db.execute("INSERT OR IGNORE INTO stock (id, count) VALUES (1, 100)");
    },
  }),
  actions: {
    reserveStock: async (c, quantity: number) => {
      await c.db.execute("UPDATE stock SET count = count - ? WHERE id = 1", quantity);
      return { reserved: quantity };
    }
  }
});

const payment = actor({
  actions: {
    processPayment: (c, amount: number) => ({ transactionId: "tx-123" })
  }
});

const orderProcessor = actor({
  // Processed orders are kept in memory for this example
  createVars: () => ({ orders: [] as ProcessedOrder[] }),

  actions: {
    processOrder: async (c, order: Order) => {
      const client = c.client<typeof registry>();

      // Reserve the stock
      const inventoryHandle = client.inventory.getOrCreate(["main"]);
      await inventoryHandle.reserveStock(order.quantity);

      // Process payment through payment actor
      const paymentHandle = client.payment.getOrCreate([order.customerId]);
      const result = await paymentHandle.processPayment(order.amount);

      // Record the processed order
      c.vars.orders.push({ ...order, status: "completed", paymentResult: result });

      return { success: true, orderId: order.id };
    }
  }
});

const registry = setup({ use: { inventory, payment, orderProcessor } });
