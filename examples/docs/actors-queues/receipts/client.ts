import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>("http://localhost:6420");
const handle = client.counter.getOrCreate(["main"]);

const receipt = await handle.send(
  "increment",
  { amount: 5 },
  { dedupeKey: "increment-5" },
);

console.log(receipt.id, receipt.deduplicated);
console.log(await receipt.status());

// A receipt can be reconstructed later from its saved ID.
console.log(await handle.receipt(receipt.id).status());
