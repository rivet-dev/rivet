import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const chatRoom = actor({
	state: { messages: [] as string[] },
	actions: {
		getRoomInfo: (c) => ({ org: c.key[0], room: c.key[1] }),
	},
});

const registry = setup({ use: { chatRoom } });
const client = createClient<typeof registry>("http://localhost:6420");

// Compound key: [org, room]
client.chatRoom.getOrCreate(["org-acme", "general"]);

// Access key inside actor via c.key
