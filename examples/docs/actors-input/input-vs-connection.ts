import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

interface RoomInput { roomName: string; isPrivate: boolean; }

const chatRoom = actor({
  createState: (c, input: RoomInput) => ({ name: input.roomName, isPrivate: input.isPrivate }),
  createConnState: (c, params: { userId: string; displayName: string }) => ({
    userId: params.userId,
    displayName: params.displayName,
  }),
  actions: {}
});

const registry = setup({ use: { chatRoom } });
const client = createClient<typeof registry>("http://localhost:6420");

// Actor creation with input
const room = await client.chatRoom.create(["room-123"], {
  input: {
    roomName: "General Discussion",
    isPrivate: false,
  },
});
