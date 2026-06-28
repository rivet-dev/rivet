import { actor } from "rivetkit";

const myActor = actor({
    state: {},
    onWebSocket: (c, websocket) => {
        if (c.request) {
            const url = new URL(c.request.url);
            console.log(url.pathname); // e.g., "/admin"
            console.log(url.searchParams.get("foo")); // e.g., "bar"
        }
    },
    actions: {}
});
