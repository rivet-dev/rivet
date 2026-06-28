import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>({
  endpoint: "https://my-namespace:pk_...@api.rivet.dev",
});
const counter = client.counter.getOrCreate(["my-counter"]);
const count = await counter.increment(1);
