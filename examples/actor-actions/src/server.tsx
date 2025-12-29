/** @jsx jsx */
/** @jsxImportSource hono/jsx */

import { registry } from "./registry";
import { Hono } from "hono";

const app = new Hono();

app.get("/", async (c) => {
	 return c.html(
    <html>
      <head>
        {import.meta.env.PROD ? (
          <script type='module' src='/static/client.js'></script>
        ) : (
          <script type='module' src='/frontend/main.tsx'></script>
        )}
      </head>
      <body>
        <div id="root"></div>
      </body>
    </html>
  )
});

// FIXME: use registry.serve()
app.mount("/api/rivet", registry.start().fetch);

export default app;
