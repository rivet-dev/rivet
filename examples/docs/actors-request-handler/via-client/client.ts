import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>("http://localhost:6420");

const actor = client.counter.getOrCreate(["my-counter"]);

// .fetch() is WinterTC compliant, it accepts standard Request and returns standard Response
const response = await actor.fetch("/");
const data = await response.json();
console.log(data); // { count: 0 }
