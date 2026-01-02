import { registry } from "./actors";

// OPTION A:
export default registry.serve();

// // OPTION B:
// import { Hono } from "hono";
//
// const app = new Hono();
//
// app.get("/api/foo", (c) => c.text("bar"));
//
// app.all("/api/rivet/*", (c) => registry.handler(c.req.raw));
//
// export default app;
