import { createClient } from "rivetkit/client";

const client = createClient();
const conn = client.chatRoom.getOrCreate(["general"]).connect();
conn.on("message", (msg) => console.log(msg));
conn.once("gameOver", () => console.log("done"));
