import { actor } from "rivetkit";

const counter = actor({
state: { count: 0 },
actions: {
increment: (c) => c.state.count += 1,
},
});
