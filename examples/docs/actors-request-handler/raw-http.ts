import { actor } from "rivetkit";

export const counterActor = actor({
    state: {
        count: 0,
    },
    // WinterTC compliant - accepts standard Request and returns standard Response
    onRequest: (c, request) => {
        const url = new URL(request.url);

        if (request.method === "GET" && url.pathname === "/count") {
            return Response.json({ count: c.state.count });
        }

        if (request.method === "POST" && url.pathname === "/increment") {
            c.state.count++;
            return Response.json({ count: c.state.count });
        }

        return new Response("Not Found", { status: 404 });
    },
    actions: {}
});
