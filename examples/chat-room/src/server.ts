import { registry } from "./actors";

// OPTION A:
// export default registry.handler({ serveManager: false });

// OPTION B:
import { Hono } from "hono";
const app = new Hono();
app.get("/api/foo", (c) => c.text("bar"));
app.mount("/api/rivet", registry.handler({ serveManager: false }).fetch, { replaceRequest: false });
export default app;
