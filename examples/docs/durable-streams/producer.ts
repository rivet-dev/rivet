import { DurableStream, IdempotentProducer } from "@durable-streams/client";

const stream = new DurableStream({
	url: "http://127.0.0.1:8642/durable-streams/v1/stream/chat/session-1",
	contentType: "application/json",
});

// Keep the producer ID the same across restarts. With autoClaim, a restarted
// agent takes over and the old one can no longer write.
const producer = new IdempotentProducer(stream, "agent-session-1", {
	autoClaim: true,
	onError: (error) => console.error("append failed", error),
});

for (const token of ["It's", " sunny", " today"]) {
	producer.append(JSON.stringify({ token }));
}

// Wait until every token is stored, then close the stream
await producer.flush();
await producer.close();
