import { stream } from "@durable-streams/client";

const url = "http://127.0.0.1:8642/durable-streams/v1/stream/chat/session-1";

// "-1" starts at the beginning. Pass a saved offset to pick up where you left off.
const savedOffset: string = "-1";

const res = await stream<{ token: string }>({ url, offset: savedOffset, live: "sse" });
res.subscribeJson(async (batch) => {
	for (const item of batch.items) console.log(item.token);

	// Save this to resume from the same place after a refresh
	console.log("offset", batch.offset);
});
