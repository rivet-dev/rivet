import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>("http://localhost:6420");
const handle = client.agent.getOrCreate(["main"]);

await handle.send("prompt", { prompt: "summarize latest logs" });
await handle.send("stop", { reason: "user canceled" });
