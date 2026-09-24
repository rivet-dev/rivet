import { createClient } from "rivetkit/client";
import type { registry } from "./registry";

// Calls your own backend, never the Rivet control plane directly.
async function fetchToken(): Promise<{ actorId: string; token: string }> {
	const response = await fetch("/token", {
		method: "POST",
		cache: "no-store",
	});
	if (!response.ok) throw new Error("could not get a Rivet token");
	return (await response.json()) as { actorId: string; token: string };
}

const { actorId } = await fetchToken();

const client = createClient<typeof registry>({
	endpoint: "https://api.rivet.dev",
	namespace: "production",

	// Called whenever RivetKit needs a credential, including after one expires.
	getToken: async () => (await fetchToken()).token,
});

const profile = client.userProfile.getForId(actorId);
const conn = profile.connect();

await conn.recordVisit();
