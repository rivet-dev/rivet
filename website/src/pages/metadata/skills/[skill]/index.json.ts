import type { APIRoute } from "astro";

import {
	getSkillConfig,
	listReferenceSummaries,
	listSkillIds,
} from "../../../../metadata/skills";

export const prerender = true;

export const GET: APIRoute = async ({ params }) => {
	const skill = params.skill;
	if (!skill) {
		return new Response("skill missing", { status: 404 });
	}

	try {
		const skills = await listSkillIds();
		if (!skills.includes(skill)) {
			return new Response("skill not found", { status: 404 });
		}
		const config = await getSkillConfig(skill);
		const references = await listReferenceSummaries(config.id);
		const payload = {
			name: config.name,
			description: config.description,
			skill_url: `/metadata/skills/${config.directory}/SKILL.md`,
			generated_at: new Date().toISOString(),
			references,
		};

		return new Response(JSON.stringify(payload, null, 2), {
			headers: { "content-type": "application/json; charset=utf-8" },
		});
	} catch (error) {
		console.error(`/metadata/skills/${skill} index failed`, error);
		return new Response(JSON.stringify({ error: "failed to build skills index" }), {
			status: 500,
			headers: { "content-type": "application/json; charset=utf-8" },
		});
	}
};

export async function getStaticPaths() {
	const skills = await listSkillIds();
	return skills.map((skill) => ({
		params: { skill },
	}));
}
