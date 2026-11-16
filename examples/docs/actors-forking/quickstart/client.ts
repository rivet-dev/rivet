import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>("http://localhost:6420");

const original = client.document.getOrCreate(["doc-1"]);
await original.write("First draft");
const originalId = await original.resolve();

// The fork starts right away, on a copy of the original's database.
const copy = await client.document.fork(originalId, {
	input: { title: "Copy of doc-1" },
});
await copy.write("Second draft");

console.log(await original.read());
console.log(await copy.read());
