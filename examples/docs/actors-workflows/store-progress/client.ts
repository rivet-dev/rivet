import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>("http://localhost:6420");
const handle = client.progressActor.getOrCreate(["main"]);
const conn = handle.connect();

conn.on("progressUpdated", (progress) => {
  console.log("progress", progress);
});

await handle.send("jobs", { value: 5 });
await handle.send("jobs", { value: 7 });

console.log(await handle.getState());
