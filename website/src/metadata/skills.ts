import { getCollection } from "astro:content";
import { existsSync } from "node:fs";
import { readFile } from "node:fs/promises";
import path from "node:path";

import { DOCS_BASE_URL, normalizeSlug } from "./shared";
import skillBaseTemplate from "./skill-base.md?raw";

export const SKILL_DIRECTORY = "rivetkit-typescript";
export const SKILL_NAME = "rivetkit-typescript";
export const SKILL_DESCRIPTION =
	"Use this skill for RivetKit building, modification, debugging, testing, or performance analysis. Trigger on RivetKit imports, Rivet/actor discussion, actor-based code analysis/logs/performance. Use RivetKit for AI agents, sandboxes, collaboration/multiplayer, realtime/WebSocket, workflows, background/scheduled jobs, or CRDT/local-first sync."

if (SKILL_DESCRIPTION.length > 500) {
	throw new Error(`SKILL_DESCRIPTION must be <= 500 chars, got ${SKILL_DESCRIPTION.length}`);
}

export type SkillReference = {
	slug: string;
	fileId: string;
	title: string;
	description: string;
	docPath: string;
	canonicalUrl: string;
	sourcePath: string | null;
	tags: string[];
	markdown: string;
};

let cachedReferences: SkillReference[] | null = null;
let cachedSkillFile: string | null = null;

async function getRivetkitVersion(): Promise<string> {
	const versionFilePath = path.join(process.cwd(), "src/generated/skill-version.json");

	if (!existsSync(versionFilePath)) {
		throw new Error(
			`skill-version.json not found at ${versionFilePath}. ` +
			`Ensure the skillVersion integration runs before skill generation.`
		);
	}

	const content = await readFile(versionFilePath, "utf-8");
	const data = JSON.parse(content);
	return data.rivetkit;
}

export async function listSkillReferences(): Promise<SkillReference[]> {
	if (cachedReferences) {
		return cachedReferences;
	}

	const docs = await getCollection("docs");
	const skillDocs = docs.filter((entry) => entry.data.skill);
	const references = skillDocs.map((entry) => buildReference(entry));
	references.sort((a, b) => a.title.localeCompare(b.title));
	cachedReferences = references;
	return references;
}

export async function getReferenceByFileId(fileId: string) {
	const references = await listSkillReferences();
	return references.find((ref) => ref.fileId === fileId);
}

export async function renderSkillFile(): Promise<string> {
	if (cachedSkillFile) {
		return cachedSkillFile;
	}

	const base = skillBaseTemplate;
	const references = await listSkillReferences();
	const referenceList = buildReferenceSection(references);

	// Get the actors overview content
	const docs = await getCollection("docs");
	const actorsIndex = docs.find((entry) => entry.id.startsWith("actors/index") || entry.id === "actors");
	if (!actorsIndex) {
		throw new Error(`actors/index not found in docs collection. Available: ${docs.map(d => d.id).join(", ")}`);
	}
	if (!actorsIndex.body) {
		throw new Error(`actors/index has no body content`);
	}

	const startMarker = "{/* SKILL_OVERVIEW_START */}";
	const endMarker = "{/* SKILL_OVERVIEW_END */}";
	const startIdx = actorsIndex.body.indexOf(startMarker);
	const endIdx = actorsIndex.body.indexOf(endMarker);

	if (startIdx === -1) {
		throw new Error(`SKILL_OVERVIEW_START marker not found in actors/index.mdx`);
	}
	if (endIdx === -1) {
		throw new Error(`SKILL_OVERVIEW_END marker not found in actors/index.mdx`);
	}

	const rawOverview = actorsIndex.body.slice(startIdx + startMarker.length, endIdx).trim();
	if (!rawOverview) {
		throw new Error(`Overview content between markers is empty`);
	}

	const overviewContent = convertDocToReference(rawOverview);

	const frontmatter = [
		"---",
		`name: "${SKILL_NAME}"`,
		`description: "${SKILL_DESCRIPTION}"`,
		"---",
		"",
	].join("\n");

	if (!base.includes("<!-- OVERVIEW -->")) {
		throw new Error(`skill-base.md does not contain <!-- OVERVIEW --> marker`);
	}
	if (!base.includes("<!-- REFERENCE_INDEX -->")) {
		throw new Error(`skill-base.md does not contain <!-- REFERENCE_INDEX --> marker`);
	}

	const rivetkitVersion = await getRivetkitVersion();

	let content = base.replace("<!-- OVERVIEW -->", overviewContent);
	content = content.replace("<!-- REFERENCE_INDEX -->", referenceList);
	content = content.replace(/\{\{RIVETKIT_VERSION\}\}/g, rivetkitVersion);
	const finalFile = `${frontmatter}\n${content}\n`;
	cachedSkillFile = finalFile;
	return finalFile;
}

export async function listReferenceSummaries() {
	const references = await listSkillReferences();
	return references.map((ref) => ({
		name: ref.fileId,
		title: ref.title,
		description: ref.description,
		canonical_url: ref.canonicalUrl,
		reference_url: `/metadata/skills/${SKILL_DIRECTORY}/reference/${ref.fileId}.md`,
	}));
}

function buildReference(entry: Awaited<ReturnType<typeof getCollection>>[number]): SkillReference {
	const slug = normalizeSlug(entry.id);
	const docPath = slug ? `/docs/${slug}` : "/docs";
	const canonicalUrl = `${DOCS_BASE_URL}${slug ? `/${slug}` : ""}`;
	const fileId = createFileId(slug);
	const body = entry.body ?? "";

	return {
		slug,
		fileId,
		title: entry.data.title,
		description: entry.data.description,
		docPath,
		canonicalUrl,
		sourcePath: entry.filePath ?? null,
		tags: slug.split("/").filter(Boolean),
		markdown: convertDocToReference(body),
	};
}

function createFileId(slug: string) {
	// Preserve path structure instead of flattening to dashes
	if (!slug) return "index";
	return slug;
}

function convertDocToReference(body: string) {
	const { replaced, restore } = extractCodeBlocks(body ?? "");
	let text = replaced;

	text = text.replace(/^[ \t]*import\s+[^;]+;?\s*$/gm, "");
	text = text.replace(/^[ \t]*export\s+[^;]+;?\s*$/gm, "");
	text = text.replace(/\{\/\*[\s\S]*?\*\/\}/g, "");

	text = stripWrapperTags(text, "Steps");
	text = stripWrapperTags(text, "Tabs");
	text = stripWrapperTags(text, "CardGroup");
	text = stripWrapperTags(text, "CodeGroup");

	text = formatHeadingBlocks(text, "Step", "Step");
	text = formatHeadingBlocks(text, "Tab", "Tab");
	text = formatCards(text);

	text = applyCallouts(text, "Tip");
	text = applyCallouts(text, "Note");
	text = applyCallouts(text, "Warning");
	text = applyCallouts(text, "Info");
	text = applyCallouts(text, "Callout");

	text = text.replace(/<Card[^>]*>/gi, "").replace(/<\/Card>/gi, "");
	text = text.replace(/<Steps[^>]*>/gi, "").replace(/<\/Steps>/gi, "");
	text = text.replace(/<Tabs[^>]*>/gi, "").replace(/<\/Tabs>/gi, "");
	text = text.replace(/<Step[^>]*>/gi, "").replace(/<\/Step>/gi, "");
	text = text.replace(/<Tab[^>]*>/gi, "").replace(/<\/Tab>/gi, "");

	text = text.replace(/<[A-Z][A-Za-z0-9]*[^>]*>/g, "").replace(/<\/[A-Z][A-Za-z0-9]*>/g, "");
	text = text.replace(/\n{3,}/g, "\n\n");

	return restore(text).trim();
}

function extractCodeBlocks(input: string) {
	const blocks: string[] = [];
	const replaced = input.replace(/```[\s\S]*?```/g, (match) => {
		const token = `@@CODE_BLOCK_${blocks.length}@@`;
		blocks.push(match);
		return token;
	});

	return {
		replaced,
		restore: (value: string) => value.replace(/@@CODE_BLOCK_(\d+)@@/g, (_, index) => blocks[Number(index)] ?? ""),
	};
}

function stripWrapperTags(input: string, tag: string) {
	const open = new RegExp(`<${tag}[^>]*>`, "gi");
	const close = new RegExp(`</${tag}>`, "gi");
	return input.replace(open, "\n").replace(close, "\n");
}

function formatHeadingBlocks(input: string, tag: string, fallback: string) {
	const withTitles = input.replace(
		new RegExp(`<${tag}[^>]*title=(?:"([^"]+)"|'([^']+)')[^>]*>`, "gi"),
		(_, doubleQuoted, singleQuoted) => `\n### ${(doubleQuoted ?? singleQuoted ?? fallback).trim()}\n\n`,
	);
	const withFallback = withTitles.replace(new RegExp(`<${tag}[^>]*>`, "gi"), `\n### ${fallback}\n\n`);
	return withFallback.replace(new RegExp(`</${tag}>`, "gi"), "\n");
}

function formatCards(input: string) {
	return input.replace(/<Card([^>]*)>([\s\S]*?)<\/Card>/gi, (_, attrs, content) => {
		const title = getAttributeValue(attrs, "title") ?? "Resource";
		const href = getAttributeValue(attrs, "href");
		const summary = collapseWhitespace(stripHtml(content));
		const link = href ? `[${title}](${href})` : title;
		const suffix = summary ? ` — ${summary}` : "";
		return `\n- ${link}${suffix}\n\n`;
	});
}

function applyCallouts(input: string, tag: string) {
	const regex = new RegExp(`<${tag}[^>]*>([\s\S]*?)</${tag}>`, "gi");
	return input.replace(regex, (_, content) => {
		const label = tag.toUpperCase();
		const text = collapseWhitespace(stripHtml(content));
		return `\n> **${label}:** ${text}\n\n`;
	});
}

function getAttributeValue(attrs: string, name: string) {
	const regex = new RegExp(`${name}=(?:"([^"]+)"|'([^']+)')`, "i");
	const match = attrs.match(regex);
	if (!match) return undefined;
	return (match[1] ?? match[2] ?? "").trim();
}

function stripHtml(value: string) {
	return value.replace(/<[^>]+>/g, " ").replace(/\s+/g, " ").trim();
}

function collapseWhitespace(value: string) {
	return value.replace(/\s+/g, " ").trim();
}

function buildReferenceSection(references: SkillReference[]) {
	// Group by top-level segment only (actors, clients, general)
	const groups = new Map<string, SkillReference[]>();

	for (const ref of references) {
		const top = resolveGroup(ref).top;
		if (!groups.has(top)) {
			groups.set(top, []);
		}
		groups.get(top)!.push(ref);
	}

	const lines: string[] = [];
	const sortedTop = [...groups.keys()].sort((a, b) => a.localeCompare(b));

	for (const topKey of sortedTop) {
		const topTitle = formatSegment(topKey);
		lines.push(`### ${topTitle}`, "");

		const entries = groups.get(topKey)!;
		entries.sort((a, b) => a.title.localeCompare(b.title));
		for (const entry of entries) {
			lines.push(`- [${entry.title}](reference/${entry.fileId}.md)`);
		}
		lines.push("");
	}

	return lines.join("\n").trim();
}

function resolveGroup(ref: SkillReference) {
	const segments = (ref.slug ?? "").split("/").filter(Boolean);
	const top = segments[0] ?? "general";
	const sub = segments[1] ?? "";
	return { top, sub };
}

function formatHeading(top: string, sub: string) {
	const topTitle = formatSegment(top);
	const subTitle = formatSegment(sub);
	return subTitle ? `${topTitle} > ${subTitle}` : topTitle;
}

function formatSegment(value: string) {
	if (!value) return "";
	const normalized = value === "index" ? "overview" : value;
	return normalized
		.split("-")
		.map((part) => part.charAt(0).toUpperCase() + part.slice(1))
		.join(" ");
}
