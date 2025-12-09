"use server";

import { freestyle } from "@/lib/freestyle";

export async function requestDevServer({ repoId }: { repoId: string }) {
	if (!repoId) {
		throw new Error("Repo ID is required");
	}

	const { codeServerUrl, ephemeralUrl } = await freestyle.requestDevServer({
		repoId,
	});

	return {
		codeServerUrl,
		ephemeralUrl,
	};
}
