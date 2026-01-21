import type { APIRoute } from "astro";
import { getCollection } from "astro:content";
import crypto from "node:crypto";
import fs from "node:fs/promises";
import path from "node:path";

import { DOCS_BASE_URL, PROJECT_ROOT, normalizeSlug } from "../../metadata/shared";

const CURATED_LIMIT = 50;

export const prerender = true;

export const GET: APIRoute = async () => {
	try {
		const metadata = await buildMetadata();
		return new Response(JSON.stringify(metadata), {
			headers: {
				"content-type": "application/json; charset=utf-8",
				"cache-control": "public, max-age=0, must-revalidate",
			},
		});
	} catch (error) {
		console.error("/metadata/docs.json failed", error);
		return new Response(JSON.stringify({ error: "failed to build metadata" }), {
			status: 500,
			headers: { "content-type": "application/json; charset=utf-8" },
		});
	}
};

async function buildMetadata(): Promise<MetadataPayload> {
	const docs = await getCollection("docs");
	const pages: PageRecord[] = [];
	const sections: SectionRecord[] = [];

	for (const entry of docs) {
		const slug = normalizeSlug(entry.id);
		const slugId = slug || "index";
		const resourceUri = `docs://page/${slugId}`;
		const canonicalPath = slug ? `/docs/${slug}` : "/docs";
		const canonicalUrl = `${DOCS_BASE_URL}${slug ? `/${slug}` : ""}`;
		const tags = slug.split("/").filter(Boolean);
		const productArea = tags[0] ?? null;
		const body = entry.body ?? "";
		const headings = ensureHeadings(extractHeadings(body), entry.data.title);
		const updatedAt = await getUpdatedAt(entry.filePath);
		const sectionRecords = splitSections(body, headings, resourceUri, canonicalUrl, updatedAt, canonicalPath);
		const plaintext = toPlainText(body);

		pages.push({
			resource_uri: resourceUri,
			slug,
			path: canonicalPath,
			canonical_url: canonicalUrl,
			title: entry.data.title,
			description: entry.data.description,
			product_area: productArea,
			tags,
			version: "current",
			lang: "en",
			updated_at: updatedAt,
			token_estimate: estimateTokens(body),
			headings: headings.map((heading) => ({
				anchor: heading.anchor,
				level: heading.level,
				title: heading.title,
				startLine: heading.startLine,
				endLine: heading.endLine ?? heading.startLine,
			})),
			markdown: body,
			plaintext,
			skill: entry.data.skill,
		});

		sections.push(...sectionRecords);
	}

	const sortedPages = pages.sort((a, b) => a.slug.localeCompare(b.slug));
	const sortedSections = sections.sort((a, b) => a.resource_uri.localeCompare(b.resource_uri));
	const llmsFull = sortedPages.map((page) => page.canonical_url);
	const llmsCurated = sortedPages
		.filter((page) => page.slug.split("/").filter(Boolean).length <= 1)
		.slice(0, CURATED_LIMIT)
		.map((page) => page.canonical_url);

	return {
		version: {
			content_hash: createHash(sortedPages, sortedSections),
			generated_at: new Date().toISOString(),
		},
		pages: sortedPages,
		sections: sortedSections,
		llms: llmsCurated,
		llms_full: llmsFull,
	};
}

async function getUpdatedAt(filePath?: string) {
	if (!filePath) return new Date().toISOString();
	const absolute = path.resolve(PROJECT_ROOT, filePath);
	try {
		const stats = await fs.stat(absolute);
		return stats.mtime.toISOString();
	} catch {
		return new Date().toISOString();
	}
}

function ensureHeadings(headings: HeadingRecord[], fallbackTitle: string) {
	if (headings.length > 0) {
		return headings;
	}
	return [
		{
			anchor: slugifyText(fallbackTitle),
			level: 1,
			title: fallbackTitle,
			startLine: 1,
			endLine: undefined,
		},
	];
}

function extractHeadings(markdown: string) {
	const lines = markdown.split(/\r?\n/);
	const records: HeadingRecord[] = [];

	for (let i = 0; i < lines.length; i += 1) {
		const line = lines[i];
		const match = line.match(/^(#{1,6})\s+(.*)$/);
		if (!match) continue;
		const level = match[1]?.length ?? 1;
		const title = cleanInlineText(match[2] ?? "").trim();
		if (!title) continue;
		records.push({
			anchor: slugifyText(title),
			level,
			title,
			startLine: i + 1,
		});
	}

	records.sort((a, b) => a.startLine - b.startLine);
	for (let i = 0; i < records.length; i += 1) {
		const current = records[i];
		const next = records[i + 1];
		current.endLine = next ? next.startLine - 1 : lines.length;
	}

	return records;
}

function splitSections(
	body: string,
	headings: HeadingRecord[],
	resourceUri: string,
	canonicalUrl: string,
	updatedAt: string,
	canonicalPath: string,
) {
	const lines = body.split(/\r?\n/);
	const records: SectionRecord[] = [];

	for (const heading of headings) {
		const start = Math.max(heading.startLine - 1, 0);
		const end = typeof heading.endLine === "number" ? heading.endLine : lines.length;
		const content = lines.slice(start, end).join("\n").trim();
		const snippet = toSnippet(content);
		const uri = `${resourceUri}#section=${heading.anchor}`;

		records.push({
			resource_uri: uri,
			parent_uri: resourceUri,
			title: heading.title,
			anchor: heading.anchor,
			canonical_url: `${canonicalUrl}#${heading.anchor}`,
			snippet,
			content,
			updated_at: updatedAt,
			token_estimate: estimateTokens(content),
			path: canonicalPath,
			start_line: heading.startLine,
			end_line: end,
		});
	}

	return records;
}

function toPlainText(markdown: string) {
	return markdown
		.replace(/```[\s\S]*?```/g, " ")
		.replace(/`([^`]+)`/g, " $1 ")
		.replace(/!\[[^\]]*]\([^)]+\)/g, " ")
		.replace(/\[[^\]]*]\(([^)]+)\)/g, " $1 ")
		.replace(/[*_~`>#-]/g, " ")
		.replace(/<[^>]+>/g, " ")
		.replace(/\s+/g, " ")
		.trim();
}

function toSnippet(content: string) {
	const text = content.replace(/\s+/g, " ").trim();
	const limit = 220;
	return text.length > limit ? `${text.slice(0, limit - 1)}…` : text;
}

function slugifyText(value: string) {
	return (
		value
			.normalize("NFKD")
			.replace(/[\u0300-\u036f]/g, "")
			.toLowerCase()
			.replace(/[^a-z0-9]+/g, "-")
			.replace(/^-+|-+$/g, "") || "section"
	);
}

function cleanInlineText(value: string) {
	return value
		.replace(/`([^`]+)`/g, "$1")
		.replace(/!\[[^\]]*]\([^)]+\)/g, "")
		.replace(/\[[^\]]*]\(([^)]+)\)/g, "$1")
		.replace(/[*_~]/g, "")
		.replace(/<[^>]+>/g, "")
		.trim();
}

function estimateTokens(input: string) {
	const words = input.split(/\s+/).filter(Boolean).length;
	return Math.max(1, Math.round(words * 1.3));
}

function createHash(pages: PageRecord[], sections: SectionRecord[]) {
	return crypto.createHash("sha256").update(JSON.stringify({ pages, sections })).digest("hex");
}

interface HeadingRecord {
	anchor: string;
	level: number;
	title: string;
	startLine: number;
	endLine?: number;
}

interface PageRecord {
	resource_uri: string;
	slug: string;
	path: string;
	canonical_url: string;
	title: string;
	description: string;
	product_area: string | null;
	tags: string[];
	version: string;
	lang: string;
	updated_at: string;
	token_estimate: number;
	headings: Array<{
		anchor: string;
		level: number;
		title: string;
		startLine: number;
		endLine: number;
	}>;
	markdown: string;
	plaintext: string;
	skill: boolean;
}

interface SectionRecord {
	resource_uri: string;
	parent_uri: string;
	title: string;
	anchor: string;
	canonical_url: string;
	snippet: string;
	content: string;
	updated_at: string;
	token_estimate: number;
	path: string;
	start_line: number;
	end_line: number;
}

interface MetadataPayload {
	version: {
		content_hash: string;
		generated_at: string;
	};
	pages: PageRecord[];
	sections: SectionRecord[];
	llms: string[];
	llms_full: string[];
}
