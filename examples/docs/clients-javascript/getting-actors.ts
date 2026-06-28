import { createClient } from "rivetkit/client";

const client = createClient();
const room = client.chatRoom.getOrCreate(["room-42"]);
const existing = client.chatRoom.get(["room-42"]);

const created = await client.game.create(["game-1"], {
  input: { mode: "ranked" },
});

const byId = client.chatRoom.getForId("actor-id");
const resolvedId = await room.resolve();
