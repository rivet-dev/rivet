import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>("http://localhost:6420");
const handle = client.counter.getOrCreate(["main"]);

const result = await handle.send(
  "increment",
  { amount: 5 },
  { wait: true, timeout: 5_000 },
);

if (result.status === "completed") {
  console.log(result.response); // { value: 5 }
} else if (result.status === "timedOut") {
  console.log("timed out");
}
