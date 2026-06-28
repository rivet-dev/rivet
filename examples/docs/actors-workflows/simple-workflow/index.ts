import { actor, setup } from "rivetkit";
import { type WorkflowStepContextOf, workflow } from "rivetkit/workflow";

export const invoiceActor = actor({
  state: {
    invoiceId: null as string | null,
    subtotal: 0,
    tax: 0,
    total: 0,
    status: "idle" as "idle" | "complete",
  },
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
    getState: (c) => c.state,
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
  ctx.state.invoiceId = invoice.id;
  ctx.state.subtotal = subtotal;
  ctx.state.tax = tax;
  ctx.state.total = total;
  ctx.state.status = "complete";
}

export const registry = setup({ use: { invoiceActor } });
