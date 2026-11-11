import { createClient } from "rivetkit/client";
import type { registry } from "./quickstart/index";

const client = createClient<typeof registry>();
const sourceId = await client.document.getOrCreate(["doc-1"]).resolve();

// Fork doc-1 into a new actor with its own key. `input` reaches its onFork.
const draft = await client.document.fork(sourceId, {
	key: ["doc-1-draft"],
	input: { title: "Draft of doc-1" },
});

// The returned handle calls the new actor like any other.
await draft.write("A new opening paragraph");
console.log(await draft.read());
