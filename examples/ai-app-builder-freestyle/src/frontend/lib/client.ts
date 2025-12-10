import { createClient } from "rivetkit/client";
import type { registry } from "../../backend/actors";

const BACKEND_URL = import.meta.env.VITE_BACKEND_URL ?? "http://localhost:6420";

export const client = createClient<typeof registry>({
	endpoint: BACKEND_URL,
});
