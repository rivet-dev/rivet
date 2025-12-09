import { createClient } from "rivetkit/client";
import type { registry } from "./registry";

// Server-side client for use in server actions
export function getRivetClient() {
	return createClient<typeof registry>({
		endpoint: process.env.RIVET_ENDPOINT ?? "http://localhost:3000/api/rivet",
		namespace: process.env.RIVET_NAMESPACE,
		token: process.env.RIVET_TOKEN,
	});
}
