import type { APIRoute } from "astro";

import {
	getReferenceByFileId,
	listSkillIds,
	listSkillReferences,
} from "../../../../../metadata/skills";

export const prerender = true;

export const GET: APIRoute = async ({ params }) => {
	const skill = params.skill;
	const referenceId = params.reference;
	if (!skill || !referenceId) {
		return new Response("reference missing", { status: 404 });
	}

	let reference;
	try {
		reference = await getReferenceByFileId(skill, referenceId);
	} catch (error) {
		console.error(`/metadata/skills/${skill}/reference/${referenceId} failed`, error);
		return new Response("reference not found", { status: 404 });
	}
	if (!reference) {
		return new Response("reference not found", { status: 404 });
	}

	const header = [
		`# ${reference.title}`,
		"",
		reference.sourcePath ? `> Source: \`${reference.sourcePath}\`` : "> Source: unknown",
		`> Canonical URL: ${reference.canonicalUrl}`,
		`> Description: ${reference.description}`,
		"",
		"---",
		"",
	].join("\n");

	const body = `${header}${reference.markdown}\n\n_Source doc path: ${reference.docPath}_\n`;

	return new Response(body, {
		headers: { "content-type": "text/markdown; charset=utf-8" },
	});
};

export async function getStaticPaths() {
	const paths = [];
	for (const skill of await listSkillIds()) {
		const references = await listSkillReferences(skill);
		for (const reference of references) {
			paths.push({
				params: { skill, reference: reference.fileId },
			});
		}
	}
	return paths;
}
