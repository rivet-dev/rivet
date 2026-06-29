import { actor } from "rivetkit";

export const chatActor = actor({
    state: { messages: [] as string[] },
    onWebSocket: (c, websocket) => {
        websocket.addEventListener("open", () => {
            // Send existing messages to new connection
            websocket.send(JSON.stringify({
                type: "history",
                messages: c.state.messages,
            }));
        });

        websocket.addEventListener("message", (event) => {
            // Store message
            c.state.messages.push(event.data as string);

            // Echo message back
            websocket.send(event.data as string);

            // Manually save state since WebSocket connections are long-lived
            c.saveState({ immediate: true });
        });
    },
    actions: {}
});
