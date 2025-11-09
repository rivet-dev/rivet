import { Hono } from "hono";
import { actor, type RequestContext } from "rivetkit";

export const rawHttpActor = actor({
	state: {
		requestCount: 0,
	},
	onRequest(
		ctx: RequestContext<any, any, any, any, any, any>,
		request: Request,
	) {
		const url = new URL(request.url);
		const method = request.method;

		// Track request count
		ctx.state.requestCount++;

		// Handle different endpoints
		if (url.pathname === "/api/hello") {
			return new Response(
				JSON.stringify({ message: "Hello from actor!" }),
				{
					headers: { "Content-Type": "application/json" },
				},
			);
		}

		if (url.pathname === "/api/echo" && method === "POST") {
			return new Response(request.body, {
				headers: request.headers,
			});
		}

		if (url.pathname === "/api/state") {
			return new Response(
				JSON.stringify({
					requestCount: ctx.state.requestCount,
				}),
				{
					headers: { "Content-Type": "application/json" },
				},
			);
		}

		if (url.pathname === "/api/headers") {
			const headers = Object.fromEntries(request.headers.entries());
			return new Response(JSON.stringify(headers), {
				headers: { "Content-Type": "application/json" },
			});
		}

		// Return 404 for unhandled paths
		return new Response("Not Found", { status: 404 });
	},
	actions: {},
});

export const rawHttpNoHandlerActor = actor({
	actions: {},
});

export const rawHttpVoidReturnActor = actor({
	onRequest(ctx, request) {
		// Intentionally return void to test error handling
		return undefined as any;
	},
	actions: {},
});

export const rawHttpHonoActor = actor({
	createVars() {
		const router = new Hono();

		// Set up routes
		router.get("/", (c: any) =>
			c.json({ message: "Welcome to Hono actor!" }),
		);

		router.get("/users", (c: any) =>
			c.json([
				{ id: 1, name: "Alice" },
				{ id: 2, name: "Bob" },
			]),
		);

		router.get("/users/:id", (c: any) => {
			const id = c.req.param("id");
			return c.json({
				id: parseInt(id),
				name: id === "1" ? "Alice" : "Bob",
			});
		});

		router.post("/users", async (c: any) => {
			const body = await c.req.json();
			return c.json({ id: 3, ...body }, 201);
		});

		router.put("/users/:id", async (c: any) => {
			const id = c.req.param("id");
			const body = await c.req.json();
			return c.json({ id: parseInt(id), ...body });
		});

		router.delete("/users/:id", (c: any) => {
			const id = c.req.param("id");
			return c.json({ message: `User ${id} deleted` });
		});

		// Return the router as a var
		return { router };
	},
	onRequest(
		ctx: RequestContext<any, any, any, any, any, any>,
		request: Request,
	) {
		// Use the Hono router from vars
		return ctx.vars.router.fetch(request);
	},
	actions: {},
});
