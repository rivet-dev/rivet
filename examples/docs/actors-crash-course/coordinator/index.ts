import { actor, setup } from "rivetkit";

// Coordinator: tracks chat rooms within an organization
export const chatRoomList = actor({
state: { rooms: [] as string[] },
actions: {
addRoom: async (c, name: string) => {
// Create the chat room actor
const client = c.client<typeof registry>();
await client.chatRoom.create([c.key[0], name]);
c.state.rooms.push(name);
},
listRooms: (c) => c.state.rooms,
},
});

// Data actor: handles a single chat room
export const chatRoom = actor({
state: { messages: [] as string[] },
actions: {
send: (c, msg: string) => { c.state.messages.push(msg); },
},
});

export const registry = setup({ use: { chatRoomList, chatRoom } });

registry.start();
