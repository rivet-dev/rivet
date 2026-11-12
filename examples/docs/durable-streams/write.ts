import { DurableStream } from "@durable-streams/client";

const stream = new DurableStream({
	url: "http://127.0.0.1:8642/durable-streams/v1/stream/chat/session-1",
	contentType: "application/json",
});

// Each append is one message
await stream.append(JSON.stringify({ token: "It's" }));
await stream.append(JSON.stringify({ token: " sunny" }));

// Close with a final message: readers know the reply is complete
await stream.close({ body: JSON.stringify({ token: "." }) });
