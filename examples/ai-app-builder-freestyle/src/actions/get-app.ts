"use server";

import { getRivetClient } from "@/rivet/server";

export async function getApp(appId: string) {
	const client = getRivetClient();
	const appActor = client.appStore.get([appId]);

	try {
		const info = await appActor.getInfo();
		if (!info) {
			return null;
		}

		const messages = await appActor.getMessages();
		const deployments = await appActor.getDeployments();

		return {
			info,
			messages,
			deployments,
		};
	} catch {
		return null;
	}
}
