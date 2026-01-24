import type { APIRoute } from "astro";

import { getReferenceByFileId, listSkillReferences } from "../../../../../metadata/skills";

export const prerender = true;

export const GET: APIRoute = async ({ params }) => {
	const referenceId = params.reference;
	if (!referenceId) {
		return new Response("reference missing", { status: 404 });
	}

	const reference = await getReferenceByFileId(referenceId);
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
	const references = await listSkillReferences();
	return references.map((reference) => ({
		params: { reference: reference.fileId },
	}));
}
