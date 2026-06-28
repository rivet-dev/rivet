import { Hono } from "hono";
import type { WSContext, WSMessageReceive } from "hono/ws";
import { upgradeWebSocket } from "hono/bun";
import { createClient } from "rivetkit/client";
import { actor, setup } from "rivetkit";

const chatActor = actor({
    state: { messages: [] as string[] },
    onWebSocket: (c, websocket) => {
        websocket.addEventListener("message", (event) => {
            c.state.messages.push(event.data as string);
            websocket.send(event.data as string);
        });
    },
    actions: {}
});

const registry = setup({ use: { chat: chatActor } });
const client = createClient<typeof registry>("http://localhost:6420");

const app = new Hono();

// Proxy WebSocket connections to actor's onWebSocket handler
app.get("/ws/:id", upgradeWebSocket(async (c) => {
    const actorId = c.req.param("id");
    const actorHandle = client.chat.get([actorId]);
    const actorWs = await actorHandle.webSocket("/");

    return {
        onOpen: (evt: Event, ws: WSContext) => {
            actorWs.addEventListener("message", (event: MessageEvent) => {
                ws.send(event.data);
            });
            actorWs.addEventListener("close", () => {
                ws.close();
            });
        },
        onMessage: (evt: MessageEvent<WSMessageReceive>, ws: WSContext) => {
            actorWs.send(evt.data as string);
        },
        onClose: () => {
            actorWs.close();
        },
    };
}));

export default app;
