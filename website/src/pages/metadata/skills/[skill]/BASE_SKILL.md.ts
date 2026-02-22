import type { APIRoute } from "astro";

import { getSkillConfig, listSkillIds, renderSkillFile } from "../../../../metadata/skills";

export const prerender = true;

export const GET: APIRoute = async ({ params }) => {
	const skill = params.skill;
	if (!skill) {
		return new Response("skill missing", { status: 404 });
	}

	const config = await getSkillConfig(skill);
	if (!config.baseSkillId) {
		return new Response("no base skill", { status: 404 });
	}

	const file = await renderSkillFile(config.baseSkillId);
	return new Response(file, {
		headers: { "content-type": "text/markdown; charset=utf-8" },
	});
};

export async function getStaticPaths() {
	const paths = [];
	for (const skill of await listSkillIds()) {
		const config = await getSkillConfig(skill);
		if (config.baseSkillId) {
			paths.push({ params: { skill } });
		}
	}
	return paths;
}
