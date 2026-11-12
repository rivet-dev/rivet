import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const chatRoom = actor({ actions: {} });
const userProfile = actor({ actions: {} });

const registry = setup({ use: { chatRoom, userProfile } });
const client = createClient<typeof registry>("http://localhost:6420");

// These are different actors, same key is fine
const userChat = client.chatRoom.getOrCreate(["user-123"]);
const userProfileHandle = client.userProfile.getOrCreate(["user-123"]);
