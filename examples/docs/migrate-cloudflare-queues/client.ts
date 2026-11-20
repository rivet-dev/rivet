import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>("http://localhost:6420");

// docs:start send
// Each queue is one actor. The key names the queue.
const emails = client.emailQueue.getOrCreate(["emails"]);

await emails.enqueue({ to: "ada@example.com", subject: "Welcome" });
// docs:end send

// docs:start batch
await emails.enqueueBatch([
	{ to: "ada@example.com", subject: "Receipt" },
	{ to: "grace@example.com", subject: "Receipt" },
]);
// docs:end batch

// docs:start delay
// Deliver in 60 seconds, like send(body, { delaySeconds: 60 })
await emails.enqueue(
	{ to: "ada@example.com", subject: "Reminder" },
	{ delayMs: 60_000 },
);
// docs:end delay

// docs:start shard
// Spread load across several queue actors. Each shard drains independently,
// and jobs for the same user always go to the same shard.
const SHARDS = 8;

function shardFor(userId: string): string {
	let hash = 0;
	for (const char of userId) hash = (hash * 31 + char.charCodeAt(0)) >>> 0;
	return String(hash % SHARDS);
}

const shard = client.emailQueue.getOrCreate(["emails", shardFor("user-42")]);
await shard.enqueue({ to: "user-42@example.com", subject: "Welcome" });
// docs:end shard

// docs:start admin
await emails.pause(); // like `wrangler queues pause-delivery`
await emails.resume(); // like `wrangler queues resume-delivery`
console.log("backlog", await emails.backlog());

for (const letter of await emails.deadLetters()) {
	await emails.redrive(letter.id);
}
// docs:end admin

// docs:start pull
const jobs = client.pullQueue.getOrCreate(["jobs"]);
await jobs.publish({ task: "resize", imageId: "img-1" });

const messages = await jobs.pull(10);
for (const message of messages) {
	console.log("processing", message.body);
}
await jobs.ack(messages.map((message) => message.leaseId));

// The same queue over HTTP, for producers and consumers outside JavaScript
await jobs.fetch("/messages", {
	method: "POST",
	headers: { "content-type": "application/json" },
	body: JSON.stringify({ body: { task: "resize", imageId: "img-2" } }),
});
// docs:end pull
