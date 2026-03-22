import { createClient } from "rivetkit/client";
import type { registry } from "../src/actors/index.ts";

export function makeClient() {
	return createClient<typeof registry>("http://localhost:6420");
}

export type GameClient = ReturnType<typeof makeClient>;
