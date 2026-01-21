import { handle } from "hono/vercel";
import app from "../src/server.ts";

export default function handler(req: Request) {
    console.log("Received request:", req.method, req.url);
    return app.fetch(req);
}