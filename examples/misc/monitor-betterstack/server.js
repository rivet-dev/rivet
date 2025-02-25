import { Hono } from "hono";
import { logger } from "hono/logger";
import { serve } from "@hono/node-server";

setTimeout(() => {
	Object.keys(process.env).forEach((key) => {
		console.log(`${key}: ${process.env[key]}`);
	});
}, 2000);

const app = new Hono();
app.use("*", logger());

app.get("/", (c) => c.text("Hello, world!"));
app.get("/health", (c) => c.json({ status: "ok" }));

const port = parseInt(process.env.PORT_HTTP || "8080");
console.log(`Server starting on port ${port}`);

// Use the Node.js server adapter
const server = serve({
	fetch: app.fetch,
	port: port,
});

// Handle graceful shutdown
process.on("SIGTERM", () => {
	console.log("SIGTERM received, shutting down gracefully");
	server.close(() => {
		console.log("Server closed");
		process.exit(0);
	});
});
