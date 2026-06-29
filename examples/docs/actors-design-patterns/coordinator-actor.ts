import { actor, setup } from "rivetkit";

// Data actor: handles messages and connections
const chatRoom = actor({
  state: { messages: [] as { sender: string; text: string }[] },
  actions: {
    sendMessage: (c, sender: string, text: string) => {
      const message = { sender, text };
      c.state.messages.push(message);
      c.broadcast("newMessage", message);
      return message;
    },
    getHistory: (c) => c.state.messages,
  },
});

// Coordinator: indexes chat rooms
const chatRoomList = actor({
  state: { chatRoomIds: [] as string[] },
  actions: {
    createChatRoom: async (c, name: string) => {
      const client = c.client<typeof registry>();
      // Create the chat room actor and get its ID
      const handle = await client.chatRoom.create([name]);
      const actorId = await handle.resolve();
      // Track it in the list
      c.state.chatRoomIds.push(actorId);
      return actorId;
    },
    listChatRooms: (c) => c.state.chatRoomIds,
  },
});

const registry = setup({
  use: { chatRoom, chatRoomList },
});
