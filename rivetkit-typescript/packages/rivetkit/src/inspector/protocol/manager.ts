import { hc } from "hono/client";
import type { createManagerInspectorRouter } from "../manager";

type ManagerInspectorRouter = ReturnType<typeof createManagerInspectorRouter>;
const client = hc<ManagerInspectorRouter>("");
export type ManagerInspectorClient = typeof client;

export const createManagerInspectorClient = (
	...args: Parameters<typeof hc>
): ManagerInspectorClient => hc<ManagerInspectorRouter>(...args);
