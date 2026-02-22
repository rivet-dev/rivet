import { transformerNotationFocus } from "@shikijs/transformers";
import { slugifyWithCounter } from "@sindresorhus/slugify";
import * as acorn from "acorn";
import { toString as mdastToString } from "mdast-util-to-string";
import { mdxAnnotations } from "mdx-annotations";
import rehypeMermaid from "rehype-mermaid";
import * as shiki from "shiki";
import { visit } from "unist-util-visit";
import theme from "../lib/textmate-code-theme";

function rehypeParseCodeBlocks() {
	return (tree) => {
		visit(tree, "element", (node, _nodeIndex, parentNode) => {
			if (node.tagName === "code") {
				// Parse language from className
				if (node.properties.className) {
					parentNode.properties.language =
						node.properties.className[0]?.replace(/^language-/, "");
				}

				// Parse annotations from either mdx-annotations or plain code fence metastring.
				const infoCandidates = [
					parentNode.properties?.annotation,
					parentNode.properties?.metastring,
					parentNode.properties?.meta,
					node.properties?.annotation,
					node.properties?.metastring,
					node.properties?.meta,
				];
				const nodeData =
					node.data && typeof node.data === "object"
						? (node.data as {
							meta?: unknown;
							metastring?: unknown;
							metaString?: unknown;
						  })
						: null;
				if (nodeData) {
					infoCandidates.push(
						nodeData.meta,
						nodeData.metastring,
						nodeData.metaString,
					);
				}
				const info = infoCandidates.find(
					(value): value is string =>
						typeof value === "string" && value.trim().length > 0,
				);
				if (info && typeof info === "string") {
					const trimmed = info.trim();

					// Try parsing as JSON first (backward compatibility)
					if (trimmed.startsWith("{")) {
						try {
							const annotations = JSON.parse(trimmed);
							for (const key in annotations) {
								parentNode.properties[key] = annotations[key];
							}
							return;
						} catch {
							// Not valid JSON, fall through to new format
						}
					}

					// New format: space-separated tokens with @flags
					// Format: {title}? @nocheck? @hide?
					const tokens = trimmed.split(/\s+/);

					for (const token of tokens) {
						if (token === "@nocheck") {
							parentNode.properties.nocheck = true;
						} else if (token === "@hide") {
							parentNode.properties.hide = true;
						} else if (token && !token.startsWith("@")) {
							// Non-flag token is the title
							parentNode.properties.title = token;
						}
					}
				}
			}
		});
	};
}

/** @type {import("shiki").Highlighter} */
let highlighter;

function normalizeClassNames(value: unknown): string[] {
	if (typeof value === "string") return value.split(/\s+/).filter(Boolean);
	if (Array.isArray(value)) {
		return value.flatMap((entry) => normalizeClassNames(entry));
	}
	return [];
}

function looksLikeMermaid(code: string): boolean {
	const firstLine = code
		.split("\n")
		.map((line) => line.trim())
		.find((line) => line.length > 0);
	if (!firstLine) return false;

	return /^(sequenceDiagram|flowchart|graph|classDiagram|stateDiagram|erDiagram|journey|gantt|pie|mindmap|timeline|gitGraph|quadrantChart|requirementDiagram|C4Context|C4Container|C4Component|C4Dynamic|C4Deployment)\b/.test(
		firstLine,
	);
}

function rehypeShiki() {
	return async (tree) => {
		highlighter ??= await shiki.getSingletonHighlighter({
			themes: [theme],
			langs: [
				"bash",
				"batch",
				"cpp",
				"csharp",
				"docker",
				"gdscript",
				"html",
				"ini",
				"js",
				"json",
				"json",
				"powershell",
				"ts",
				"typescript",
				"yaml",
				"http",
				"prisma",
				"rust",
				"swift",
				"toml",
			],
		});

		visit(tree, "element", (node, _index, parentNode) => {
			if (
				node.tagName === "pre" &&
				node.children[0]?.tagName === "code"
			) {
				const codeNode = node.children[0];
				const textNode = codeNode.children[0];
				const preClassNames = normalizeClassNames(node.properties.className);
				const codeClassNames = normalizeClassNames(codeNode.properties?.className);
				const isMermaid =
					preClassNames.includes("mermaid") ||
					codeClassNames.includes("mermaid") ||
					codeClassNames.includes("language-mermaid") ||
					looksLikeMermaid(textNode.value);

				if (isMermaid) {
					node.properties.language = "mermaid";
					node.properties.className = Array.from(
						new Set([...preClassNames, "mermaid"]),
					);
					node.properties.code = textNode.value;
					return;
				}

				node.properties.code = textNode.value;

				// Default to "text" if no language specified
				const lang = node.properties.language || "text";
				node.properties.language = lang;

				try {
					const result = highlighter.codeToHtml(textNode.value, {
						lang,
						theme: theme.name,
						transformers: [transformerNotationFocus()],
					});
					// Store the highlighted HTML in a property instead of the text node
					// This prevents MDX from interpreting the HTML as JSX
					node.properties.highlightedCode = result;
				} catch (error) {
					console.error("[rehypeShiki] Error highlighting code:", error);
				}
			}
		});
	};
}

function rehypeSlugify() {
	return (tree) => {
		const slugify = slugifyWithCounter();
		visit(tree, "element", (node) => {
			if (
				(node.tagName === "h2" || node.tagName === "h3") &&
				!node.properties.id
			) {
				node.properties.id = slugify(mdastToString(node));
			}
		});
	};
}

function rehypeTableOfContents() {
	return (tree) => {
		// Headings
		const slugify = slugifyWithCounter();
		const headings: Array<{ title: string; id: string; children: Array<{ title: string; id: string; children: never[] }> }> = [];
		// find all headings, remove the first one (the title)
		visit(tree, "element", (node) => {
			if (node.tagName === "h2" || node.tagName === "h3") {
				if (node.tagName === "h3" && headings.length === 0) {
					const line = node.position?.start?.line;
					const location = typeof line === "number" ? `line ${line}` : "unknown line";
					throw new Error(
						`[rehypeTableOfContents] Found h3 before any h2 (${location}). Use h2 for top-level sections and h3 for subsections.`
					);
				}

				const parent =
					node.tagName === "h2"
						? headings
						: headings[headings.length - 1].children;
				parent.push({
					title: mdastToString(node),
					id: slugify(mdastToString(node)),
					children: [],
				});
			}
		});

		const code = `export const tableOfContents = ${JSON.stringify(headings, null, 2)};`;

		tree.children.push({
			type: "mdxjsEsm",
			value: code,
			data: {
				estree: acorn.parse(code, {
					sourceType: "module",
					ecmaVersion: "latest",
				}),
			},
		});
	};
}

// Use 'pre-mermaid' strategy for client-side rendering
// This avoids needing Playwright/Chromium in Docker/CI environments
const mermaidConfig = {
	strategy: "pre-mermaid" as const,
	mermaidConfig: { theme: "dark" },
};

export const rehypePlugins = [
	mdxAnnotations.rehype,
	[rehypeMermaid, mermaidConfig],
	rehypeParseCodeBlocks,
	rehypeShiki,
	rehypeSlugify,
	rehypeTableOfContents,
];
