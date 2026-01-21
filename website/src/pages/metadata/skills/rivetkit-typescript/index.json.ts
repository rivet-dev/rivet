import type { APIRoute } from "astro";

import {
	listReferenceSummaries,
	SKILL_DESCRIPTION,
	SKILL_DIRECTORY,
	SKILL_NAME,
} from "../../../../metadata/skills";

export const prerender = true;

export const GET: APIRoute = async () => {
	try {
		const references = await listReferenceSummaries();
		const payload = {
			name: SKILL_NAME,
			description: SKILL_DESCRIPTION,
			skill_url: `/metadata/skills/${SKILL_DIRECTORY}/SKILL.md`,
			generated_at: new Date().toISOString(),
			references,
		};

		return new Response(JSON.stringify(payload, null, 2), {
			headers: { "content-type": "application/json; charset=utf-8" },
		});
	} catch (error) {
		console.error("/metadata/skills index failed", error);
		return new Response(JSON.stringify({ error: "failed to build skills index" }), {
			status: 500,
			headers: { "content-type": "application/json; charset=utf-8" },
		});
	}
};
