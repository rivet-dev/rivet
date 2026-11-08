import { actor, event } from "rivetkit";

type Message = {
  id: string;
  userId: string;
  text: string;
  timestamp: number;
};

const chatRoom = actor({
  events: {
    messageReceived: event<Message>()
  },

  actions: {
    sendMessage: (c, userId: string, text: string) => {
      const message = {
        id: crypto.randomUUID(),
        userId,
        text,
        timestamp: Date.now()
      };

      // Broadcast to all connected clients
      c.broadcast('messageReceived', message);

      return message;
    },
  }
});
