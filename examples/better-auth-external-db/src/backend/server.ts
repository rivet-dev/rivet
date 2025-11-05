import { serve } from "@hono/node-server";
import { Hono } from "hono";
import { cors } from "hono/cors";
import { ALLOWED_PUBLIC_HEADERS } from "rivetkit";
import { auth } from "./auth";
import { registry } from "./registry";

// Start RivetKit
registry.start();

// Setup router
const app = new Hono();

app.use(
	"*",
	cors({
		origin: "http://localhost:5173",
		// Need to allow custom headers used in RivetKit
		allowHeaders: ["Authorization", ...ALLOWED_PUBLIC_HEADERS],
		allowMethods: ["POST", "GET", "OPTIONS"],
		exposeHeaders: ["Content-Length"],
		maxAge: 600,
		credentials: true,
	}),
);

// Mount Better Auth routes
app.on(["GET", "POST"], "/api/auth/**", (c) => auth.handler(c.req.raw));

serve({ fetch: app.fetch, port: 8080 });
console.log("Listening on port 8080");
