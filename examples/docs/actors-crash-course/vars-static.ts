import { actor } from "rivetkit";

const counter = actor({
state: { count: 0 },
vars: { lastAccess: 0 },
actions: {
increment: (c) => {
c.vars.lastAccess = Date.now();
return c.state.count += 1;
},
},
});
