"use client";

import { createClient } from "rivetkit/client";
import type { registry } from "./registry";

// Create a singleton client instance
export const client = createClient<typeof registry>({
	endpoint: process.env.NEXT_PUBLIC_RIVET_ENDPOINT ?? "http://localhost:3000/api/rivet",
	// Convert empty string to undefined (env vars set as VAR= become empty string, not undefined)
	namespace: process.env.NEXT_PUBLIC_RIVET_NAMESPACE || undefined,
	token: process.env.NEXT_PUBLIC_RIVET_TOKEN || undefined,
});
