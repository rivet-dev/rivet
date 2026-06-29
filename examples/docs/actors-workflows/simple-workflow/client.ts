import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>("http://localhost:6420");
const handle = client.invoiceActor.getOrCreate(["main"]);

const state = await handle.getState();
console.log(state.status, state.total);
