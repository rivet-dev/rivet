import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>("http://localhost:6420");
const handle = client.auctionActor.getOrCreate(["item-123"]);

await handle.send("bids", { amount: 100 });
console.log(await handle.getState());
