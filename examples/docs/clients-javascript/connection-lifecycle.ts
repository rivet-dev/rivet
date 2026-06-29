import { createClient } from "rivetkit/client";

const client = createClient();
const conn = client.chatRoom.getOrCreate(["general"]).connect();

conn.onOpen(() => console.log("connected"));
conn.onClose(() => console.log("disconnected"));
conn.onError((err) => console.error("error:", err));
conn.onStatusChange((status) => console.log("status:", status));

await conn.dispose();
