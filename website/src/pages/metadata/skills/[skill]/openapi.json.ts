import type { APIRoute } from "astro";
import { readFile } from "node:fs/promises";
import path from "node:path";

import {
	listSkillIds,
	skillSupportsOpenApi,
	type SkillId,
} from "../../../../metadata/skills";

export const prerender = true;

export const GET: APIRoute = async ({ params }) => {
	const skill = params.skill;
	if (!skill) {
		return new Response("skill missing", { status: 404 });
	}

	try {
		if (!skillSupportsOpenApi(skill as SkillId)) {
			return new Response("openapi not found", { status: 404 });
		}
		const openapiPath = path.join(process.cwd(), "src/generated/rivetkit-openapi.json");
		const content = await readFile(openapiPath, "utf-8");
		return new Response(content, {
			headers: { "content-type": "application/json; charset=utf-8" },
		});
	} catch (error) {
		console.error(`/metadata/skills/${skill}/openapi.json failed`, error);
		return new Response("openapi not found", { status: 404 });
	}
};

export async function getStaticPaths() {
	return listSkillIds()
		.filter((skill) => skillSupportsOpenApi(skill))
		.map((skill) => ({
			params: { skill },
		}));
}
