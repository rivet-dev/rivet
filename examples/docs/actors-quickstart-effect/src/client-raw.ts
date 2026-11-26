import { createClient } from "rivetkit/client";

const client = createClient("http://localhost:6420");

const counter = client.Counter.getOrCreate(["my-counter"]);

const count = await counter.Increment({ amount: 3 });
console.log("New count:", count);
