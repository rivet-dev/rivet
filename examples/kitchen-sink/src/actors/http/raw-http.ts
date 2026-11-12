import { Hono } from "hono";
import { actor, type RequestContext } from "rivetkit";
import { db } from "rivetkit/db";

export const rawHttpActor = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS request_stats (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					request_count INTEGER NOT NULL
				);
			`);
			await db.execute(
				"INSERT OR IGNORE INTO request_stats (id, request_count) VALUES (1, 0)",
			);
		},
	}),
	async onRequest(ctx, request: Request) {
		const url = new URL(request.url);
		const method = request.method;

		// Track request count
		const rows = await ctx.db.execute<{ request_count: number }>(
			"UPDATE request_stats SET request_count = request_count + 1 WHERE id = 1 RETURNING request_count",
		);
		const requestCount = rows[0].request_count;

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
					requestCount,
				}),
				{
					headers: { "Content-Type": "application/json" },
				},
			);
		}

		if (url.pathname === "/api/headers") {
			const headers: Record<string, string> = {};
			request.headers.forEach((value, key) => {
				headers[key] = value;
			});
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
	onRequest(_ctx, _request) {
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
				id: parseInt(id, 10),
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
			return c.json({ id: parseInt(id, 10), ...body });
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
