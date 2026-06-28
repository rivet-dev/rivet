import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>("http://localhost:6420");
const handle = client.counter.getOrCreate(["main"]);

await handle.send("increment", { amount: 1 });
await handle.send("increment", { amount: 5 });
