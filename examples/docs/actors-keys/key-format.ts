import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const counter = actor({
  vars: { count: 0 },
  actions: { increment: (c) => c.vars.count++ }
});

const chatRoom = actor({
  actions: {}
});

const registry = setup({ use: { counter, chatRoom } });
const client = createClient<typeof registry>("http://localhost:6420");

// String key
const counterHandle = client.counter.getOrCreate(["my-counter"]);

// Array key (compound key)
const chatRoomHandle = client.chatRoom.getOrCreate(["room", "general"]);
