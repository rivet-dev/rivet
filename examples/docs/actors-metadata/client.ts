import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>("http://localhost:6420");

// Connect to a chat room
const chatRoomHandle = client.chatRoom.get(["general"]);

// Get actor metadata
const metadata = await chatRoomHandle.getMetadata();
console.log("Actor metadata:", metadata);
