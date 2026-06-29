import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>("http://localhost:6420");
const handle = client.requestResponseActor.getOrCreate(["main"]);

const result = await handle.send(
  "requests",
  { value: 21 },
  { wait: true, timeout: 1_000 },
);

if (result.status === "completed") {
  const response = result.response as { doubled: number };
  console.log(response.doubled);
}
