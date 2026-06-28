import { actor, setup } from "rivetkit";

export const counter = actor({
    state: { count: 0 },
    onRequest: (c, request) => {
        if (request.method === "POST") c.state.count++;
        return Response.json(c.state);
    },
    actions: {}
});

export const registry = setup({ use: { counter } });
registry.start();
