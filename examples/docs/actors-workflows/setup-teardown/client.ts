import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>("http://localhost:6420");
const handle = client.setupRunTeardownActor.getOrCreate(["main"]);

await handle.send("work", { amount: 5 });
await handle.send("work", { amount: 3 });
await handle.send("control", { type: "stop", reason: "maintenance" });

const state = await handle.getState();
console.log(state.phase, state.total, state.stopReason);
