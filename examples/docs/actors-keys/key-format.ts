import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const counter = actor({
  state: { count: 0 },
  actions: { increment: (c) => c.state.count++ }
});

const chatRoom = actor({
  state: { messages: [] as string[] },
  actions: {}
});

const registry = setup({ use: { counter, chatRoom } });
const client = createClient<typeof registry>("http://localhost:6420");

// String key
const counterHandle = client.counter.getOrCreate(["my-counter"]);

// Array key (compound key)
const chatRoomHandle = client.chatRoom.getOrCreate(["room", "general"]);
