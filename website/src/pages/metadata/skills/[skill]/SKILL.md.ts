import type { APIRoute } from "astro";

import { listSkillIds, renderSkillFile, type SkillId } from "../../../../metadata/skills";

export const prerender = true;

export const GET: APIRoute = async ({ params }) => {
	const skill = params.skill;
	if (!skill) {
		return new Response("skill missing", { status: 404 });
	}

	try {
		const file = await renderSkillFile(skill as SkillId);
		return new Response(file, {
			headers: { "content-type": "text/markdown; charset=utf-8" },
		});
	} catch (error) {
		console.error(`/metadata/skills/${skill}/SKILL.md failed`, error);
		return new Response("skill not found", { status: 404 });
	}
};

export async function getStaticPaths() {
	return listSkillIds().map((skill) => ({
		params: { skill },
	}));
}
