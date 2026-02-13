import { createClient } from "rivetkit/client";
import type { registry } from "../src/actors/index.ts";

export function makeClient() {
	return createClient<typeof registry>({
		endpoint: `${window.location.origin}/api/rivet`,
		namespace: "default",
		runnerName: "default",
	});
}

export type GameClient = ReturnType<typeof makeClient>;
