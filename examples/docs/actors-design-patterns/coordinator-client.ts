import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const chatRoom = actor({
  state: { messages: [] as { sender: string; text: string }[] },
  actions: {
    sendMessage: (c, sender: string, text: string) => {
      const message = { sender, text };
      c.state.messages.push(message);
      return message;
    },
    getHistory: (c) => c.state.messages,
  },
});

const chatRoomList = actor({
  state: { chatRoomIds: [] as string[] },
  actions: {
    createChatRoom: async (c, name: string) => "room-id",
    listChatRooms: (c) => c.state.chatRoomIds,
  },
});

const registry = setup({ use: { chatRoom, chatRoomList } });
const client = createClient<typeof registry>("http://localhost:6420");

// Create a new chat room via coordinator
const coordinator = client.chatRoomList.getOrCreate(["main"]);
const actorId = await coordinator.createChatRoom("general");

// Get list of all chat rooms
const chatRoomIds = await coordinator.listChatRooms();

// Connect to a chat room using its ID
const chatRoomHandle = client.chatRoom.getForId(actorId);
await chatRoomHandle.sendMessage("alice", "Hello!");
const history = await chatRoomHandle.getHistory();
