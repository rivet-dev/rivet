import { Elysia } from "elysia";
import { actor, setup } from "rivetkit";

const myActor = actor({ state: {}, actions: {} });
const registry = setup({ use: { myActor } });

const app = new Elysia()
  .get("/health", () => "OK")
  .all("/api/rivet/*", ({ request }) => registry.handler(request));

export default app;
