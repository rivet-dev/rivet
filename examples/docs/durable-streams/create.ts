import { DurableStream } from "@durable-streams/client";

// One stream per chat session
const stream = await DurableStream.create({
	url: "http://127.0.0.1:8642/durable-streams/v1/stream/chat/session-1",
	contentType: "application/json",
	// Delete the stream after a day with no reads or writes
	ttlSeconds: 86400,
});

console.log("created", stream.url);
