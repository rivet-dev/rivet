import { actor, event, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

type Message = { text: string };

const chatRoom = actor({
  state: { messages: [] as string[] },
  events: {
    messageReceived: event<Message>()
  },
  actions: {
    sendMessage: (c, text: string) => {
      c.state.messages.push(text);
      c.broadcast('messageReceived', { text });
    }
  }
});

const registry = setup({ use: { chatRoom } });
const client = createClient<typeof registry>("http://localhost:6420");
const connection = client.chatRoom.getOrCreate(["general"]).connect();

// Add listener
const unsubscribe = connection.on('messageReceived', (message) => {
  console.log("Received:", message);
});

// Remove listener
unsubscribe();
