import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>("http://localhost:6420");
const handle = client.dashboardActor.getOrCreate(["main"]);

await handle.send("refresh", {});
console.log(await handle.getState());
