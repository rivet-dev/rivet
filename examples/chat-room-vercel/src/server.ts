import { Hono } from "hono";
import { registry } from "./actors.ts";

const app = new Hono();
// app.all("/api/rivet/*", (c) => {
//     console.log("[rivet] Handling request for:", c.req.url);
//   return registry.handler(c.req.raw)
// });
// app.notFound((c) => c.json({ message: "Not Found" }, 404));
// app.onError((err, c) => {
//   console.error(`${err}`)
//   return c.text('Custom Error Message', 500)
// })
app.all("*", (c) => {
    console.log("[rivet] Handling request for:", c.req.url);
//   return registry.handler(c.req.raw)
return registry.handler(c.req.raw);
})
export default app;