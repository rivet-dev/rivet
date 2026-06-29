import { actor } from "rivetkit";

const myActor = actor({
    state: {},
    onWebSocket: async (c, websocket) => {
        // Perform async operations before the connection is ready
        const metadata = await fetch("https://api.example.com/metadata").then(r => r.json());

        websocket.addEventListener("open", () => {
            // Send metadata on connection
            websocket.send(JSON.stringify({ metadata }));
        });

        websocket.addEventListener("message", (event) => {
            // Handle messages
        });
    },
    actions: {}
});
