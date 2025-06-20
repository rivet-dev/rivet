import { serve } from "@hono/node-server";
import { app } from "./app";

const PORT = process.env.PORT || 3000;
console.log(`Server starting on port ${PORT}...`);
serve({
	fetch: app.fetch,
	port: Number(PORT),
});

