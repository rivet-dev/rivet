import { stream } from "@durable-streams/client";

const url = "http://127.0.0.1:8642/durable-streams/v1/stream/chat/session-1";

// Everything written so far, then stop
const res = await stream<{ token: string }>({ url, offset: "-1", live: false });
const messages = await res.json();

console.log(messages);
console.log("read from here next:", res.offset);
