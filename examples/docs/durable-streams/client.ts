import { DurableStream } from "@durable-streams/client";

const stream = await DurableStream.create({
	url: "http://127.0.0.1:8642/durable-streams/v1/stream/demo",
	contentType: "application/json",
});

await stream.append(JSON.stringify({ message: "hello" }));

// Read from the beginning, then keep receiving new messages live
const res = await stream.stream<{ message: string }>();
res.subscribeJson(async (batch) => {
	for (const item of batch.items) console.log(item.message);
});
