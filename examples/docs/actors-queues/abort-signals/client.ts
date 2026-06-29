import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>("http://localhost:6420");
const handle = client.signalWorker.getOrCreate(["main"]);

await handle.send("jobs", { id: "job-1" });
await handle.cancelProcessing();
