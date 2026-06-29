import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>("http://localhost:6420");
const handle = client.reminderActor.getOrCreate(["main"]);

await handle.send("reminders", {
  text: "send weekly report",
  at: Date.now() + 1_000,
});

await new Promise((resolve) => setTimeout(resolve, 1_300));
console.log(await handle.getState());
