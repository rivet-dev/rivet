"use server";

import { getRivetClient } from "@/rivet/server";

export async function stopStreamAction(appId: string) {
	const client = getRivetClient();
	const streamActor = client.streamState.getOrCreate([appId]);
	await streamActor.abort();
	return { success: true };
}
