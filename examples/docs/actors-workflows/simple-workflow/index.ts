import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

export const invoiceActor = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS invoice (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          invoice_id TEXT,
          subtotal INTEGER NOT NULL,
          tax INTEGER NOT NULL,
          total INTEGER NOT NULL,
          status TEXT NOT NULL
        );
      `);
      await db.execute(
        "INSERT OR IGNORE INTO invoice (id, invoice_id, subtotal, tax, total, status) VALUES (1, NULL, 0, 0, 0, 'idle')",
      );
    },
  }),
  run: workflow(async (ctx) => {
    const subtotal = await ctx.step("load-subtotal", async (ctx) =>
      loadSubtotal(),
    );

    const tax = await ctx.step("calculate-tax", async (ctx) =>
      calculateTax(subtotal),
    );

    await ctx.step("save-invoice", async (step) =>
      saveInvoice(step, subtotal, tax),
    );
  }),
  actions: {
    getState: async (c) => {
      const rows = await c.db.execute<{
        invoiceId: string | null;
        subtotal: number;
        tax: number;
        total: number;
        status: "idle" | "complete";
      }>(
        "SELECT invoice_id AS invoiceId, subtotal, tax, total, status FROM invoice WHERE id = 1",
      );
      return rows[0];
    },
  },
});

async function loadSubtotal(): Promise<number> {
  const response = await fetch("https://api.example.com/carts/main");
  if (!response.ok) {
    throw new Error(`load subtotal failed: ${response.status}`);
  }
  const cart = (await response.json()) as { subtotal: number };
  return cart.subtotal;
}

async function calculateTax(subtotal: number): Promise<number> {
  const response = await fetch("https://api.example.com/tax/quote", {
    method: "POST",
    headers: {
      "content-type": "application/json",
    },
    body: JSON.stringify({ subtotal }),
  });
  if (!response.ok) {
    throw new Error(`tax quote failed: ${response.status}`);
  }
  const quote = (await response.json()) as { tax: number };
  return quote.tax;
}

async function saveInvoice(
  ctx: WorkflowStepContextOf<typeof invoiceActor>,
  subtotal: number,
  tax: number,
): Promise<void> {
  const total = subtotal + tax;
  const response = await fetch("https://api.example.com/invoices", {
    method: "POST",
    headers: {
      "content-type": "application/json",
    },
    body: JSON.stringify({ subtotal, tax, total }),
  });
  if (!response.ok) {
    throw new Error(`save invoice failed: ${response.status}`);
  }
  const invoice = (await response.json()) as { id: string };
  await ctx.db.execute(
    "UPDATE invoice SET invoice_id = ?, subtotal = ?, tax = ?, total = ?, status = 'complete' WHERE id = 1",
    invoice.id,
    subtotal,
    tax,
    total,
  );
}

export const registry = setup({ use: { invoiceActor } });
