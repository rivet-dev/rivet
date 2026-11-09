import { actor, UserError } from "rivetkit";

const ALLOWED_ORIGINS = [
  "http://localhost:3000",
  "https://myapp.com",
  "https://www.myapp.com"
];

const myActor = actor({
  vars: { count: 0 },

  onBeforeConnect: (c, params) => {
    // Check if origin is allowed
    //
    // This works for both HTTP & WebSocket requests
    const origin = c.request?.headers.get("origin") ?? "";
    if (!ALLOWED_ORIGINS.includes(origin)) {
      throw new UserError("Origin not allowed", { code: "origin_not_allowed" });
    }
  },

  actions: {
    increment: (c) => {
      c.vars.count++;
      return c.vars.count;
    }
  }
});
