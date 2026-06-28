import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>("http://localhost:6420");
const handle = client.workflowCounter.getOrCreate(["main"]);

await handle.send("counter", { delta: 1 });
await handle.send("counter", { delta: 2 });

const state = await handle.getState();
console.log(state.value, state.processed);
