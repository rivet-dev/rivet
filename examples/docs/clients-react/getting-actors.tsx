import { createRivetKit } from "@rivetkit/react";
import { createClient } from "rivetkit/client";

const { useActor } = createRivetKit();

function ChatRoom() {
  const room = useActor({ name: "chatRoom", key: ["room-42"] });
  return <div>{room.connStatus}</div>;
}

// For get/getOrCreate/create/getForId, use createClient
const client = createClient();
const handle = client.chatRoom.getOrCreate(["room-42"]);
const existing = client.chatRoom.get(["room-42"]);
const created = await client.game.create(["game-1"], { input: { mode: "ranked" } });
const byId = client.chatRoom.getForId("actor-id");
const resolvedId = await handle.resolve();
