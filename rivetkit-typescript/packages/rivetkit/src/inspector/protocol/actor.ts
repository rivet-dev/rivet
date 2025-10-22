import { hc } from "hono/client";
import type { createActorInspectorRouter } from "../actor";

type ActorInspectorRouter = ReturnType<typeof createActorInspectorRouter>;
const client = hc<ActorInspectorRouter>("");
export type ActorInspectorClient = typeof client;

export const createActorInspectorClient = (
	...args: Parameters<typeof hc>
): ActorInspectorClient => hc<ActorInspectorRouter>(...args);
