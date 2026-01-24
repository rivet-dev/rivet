import type { APIRoute } from "astro";
import { readFile } from "node:fs/promises";
import path from "node:path";

export const prerender = true;

export const GET: APIRoute = async () => {
	const openapiPath = path.join(process.cwd(), "src/generated/rivetkit-openapi.json");
	const content = await readFile(openapiPath, "utf-8");
	return new Response(content, {
		headers: { "content-type": "application/json; charset=utf-8" },
	});
};
