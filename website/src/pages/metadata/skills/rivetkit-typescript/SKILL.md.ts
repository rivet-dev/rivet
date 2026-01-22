import type { APIRoute } from "astro";

import { renderSkillFile } from "../../../../metadata/skills";

export const prerender = true;

export const GET: APIRoute = async () => {
	const file = await renderSkillFile();
	return new Response(file, {
		headers: { "content-type": "text/markdown; charset=utf-8" },
	});
};
