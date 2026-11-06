import { createClient } from "rivetkit/client";
import type { registry } from "./room";

const client = createClient<typeof registry>("http://localhost:6420");

// Replaces new PartySocket({ host, party: "chat", room: "general" })
const room = client.chatRoom.getOrCreate(["general"], {
	params: { name: "Ada", team: "red" },
});
const conn = room.connect();

// Replaces socket.addEventListener("message", ...) and a switch on the message type
conn.on("message", (message) => {
	console.log(`${message.sender}: ${message.text}`);
});
conn.on("joined", ({ name }) => console.log(`${name} joined`));

// Replaces socket.send(JSON.stringify({ type: "chat", text }))
await conn.sendMessage("Hello!");

// Call through the connection. A one-off call on a handle with params would
// run onConnect and onDisconnect, announcing a join and leave.
const history = await conn.getHistory();
console.log(history.length);
