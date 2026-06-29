import { actor } from "rivetkit";

const chatRoom = actor({
state: {},
connState: { visitorId: 0 },
onConnect: (c, conn) => {
conn.state.visitorId = Math.random();
},
actions: {
whoAmI: (c) => c.conn.state.visitorId,
},
});
