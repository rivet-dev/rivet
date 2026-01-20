import { handle } from "hono/vercel";
import app from "../src/server.ts";

export default handle(app);
