import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

interface ChatRoomInput {
  maxUsers: number;
  isPrivate: boolean;
  moderators: string[];
  settings: { allowImages: boolean; slowMode: boolean };
}

const chatRoom = actor({
  createState: (c, input: ChatRoomInput) => ({
    maxUsers: input.maxUsers,
    isPrivate: input.isPrivate,
    moderators: input.moderators,
    settings: input.settings,
  }),
  actions: {}
});

const registry = setup({ use: { chatRoom } });
const client = createClient<typeof registry>("http://localhost:6420");
const roomName = "general";

// Create with both key and input
const chatRoomHandle = await client.chatRoom.create(["room", roomName], {
  input: {
    maxUsers: 100,
    isPrivate: false,
    moderators: ["admin1", "admin2"],
    settings: {
      allowImages: true,
      slowMode: false
    }
  }
});
