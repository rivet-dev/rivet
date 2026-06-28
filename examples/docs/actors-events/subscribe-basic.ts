import { actor, event, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

type Message = { id: string; userId: string; text: string };

// Define the actor
const chatRoom = actor({
  state: { messages: [] as Message[] },
  events: {
    messageReceived: event<Message>()
  },
  actions: {
    sendMessage: (c, userId: string, text: string) => {
      const message = { id: crypto.randomUUID(), userId, text };
      c.state.messages.push(message);
      c.broadcast('messageReceived', message);
      return message;
    }
  }
});

const registry = setup({ use: { chatRoom } });
const client = createClient<typeof registry>("http://localhost:6420");

// Helper function for demonstration
function displayMessage(message: Message) {
  console.log("Display:", message);
}

// Get actor handle and establish connection
const chatRoomHandle = client.chatRoom.getOrCreate(["general"]);
const connection = chatRoomHandle.connect();

// Listen for events
connection.on('messageReceived', (message: Message) => {
  console.log(`${message.userId}: ${message.text}`);
  displayMessage(message);
});

// Call actions through the connection
await connection.sendMessage("user-123", "Hello everyone!");
