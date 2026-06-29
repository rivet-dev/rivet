import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const chatRoom = actor({
  state: { messages: [] as string[] },
  actions: { getRoomName: (c) => c.key[1] }
});

const registry = setup({ use: { chatRoom } });
const client = createClient<typeof registry>("http://localhost:6420");

async function connectToRoom(roomName: string) {
  // Connect to a chat room
  const chatRoomHandle = client.chatRoom.getOrCreate(["room", roomName]);

  // Get the room name from the key
  const retrievedRoomName = await chatRoomHandle.getRoomName();
  console.log("Room name:", retrievedRoomName); // e.g., "general"

  return chatRoomHandle;
}

// Usage example
const generalRoom = await connectToRoom("general");
