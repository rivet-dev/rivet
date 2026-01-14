import { transformerNotationFocus } from "@shikijs/transformers";
import { slugifyWithCounter } from "@sindresorhus/slugify";
import * as acorn from "acorn";
import { toString as mdastToString } from "mdast-util-to-string";
import { mdxAnnotations } from "mdx-annotations";
import rehypeMdxTitle from "rehype-mdx-title";
import * as shiki from "shiki";
import { visit } from "unist-util-visit";
import theme from "../lib/textmate-code-theme";
import { transformerTemplateVariables } from "./transformers";

function rehypeParseCodeBlocks() {
	return (tree) => {
		visit(tree, "element", (node, _nodeIndex, parentNode) => {
			if (node.tagName === "code") {
				// Parse language
				if (node.properties.className) {
					parentNode.properties.language =
						node.properties.className[0]?.replace(/^language-/, "");
				}
				// Parse annotations
				const info = parentNode.properties?.annotation || node.data;
				if (info) {
					let annotations = info;

					try {
						annotations = JSON.parse(annotations);
					} catch (e) {}

					if (typeof annotations === "string") {
						annotations = { title: annotations.trim() };
					}

					// Autofill is handled client-side in AutofillCodeBlock.tsx
					// Just pass through the autofill flag
					if (annotations.autofill) {
						parentNode.properties.autofill = true;
					}

					for (const key in annotations) {
						parentNode.properties[key] = annotations[key];
					}
				}
			}
		});
	};
}

/** @type {import("shiki").Highlighter} */
let highlighter;

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

				node.properties.code = textNode.value;

				// Default to "text" if no language specified
				const lang = node.properties.language || "text";
				node.properties.language = lang;

				const transformers = [transformerNotationFocus()];

				// Add template variable transformer for autofill blocks
				if (
					node.properties?.autofill ||
					parentNode.properties?.autofill
				) {
					transformers.push(transformerTemplateVariables());
				}

				try {
					const result = highlighter.codeToHtml(textNode.value, {
						lang,
						theme: theme.name,
						transformers,
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

function rehypeDescription() {
	return (tree) => {
		let description = "";
		visit(tree, "element", (node) => {
			if (node.tagName === "p" && !description) {
				description = mdastToString(node);
			}
		});

		tree.children.unshift({
			type: "mdxjsEsm",
			value: `export const description = ${JSON.stringify(description)};`,
			data: {
				estree: acorn.parse(
					`export const description = ${JSON.stringify(description)};`,
					{
						sourceType: "module",
						ecmaVersion: "latest",
					},
				),
			},
		});
	};
}

function rehypeTableOfContents() {
	return (tree) => {
		// Headings
		const slugify = slugifyWithCounter();
		const headings = [];
		// find all headings, remove the first one (the title)
		visit(tree, "element", (node) => {
			if (node.tagName === "h2" || node.tagName === "h3") {
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

export const rehypePlugins = [
	mdxAnnotations.rehype,
	rehypeParseCodeBlocks,
	rehypeShiki,
	rehypeSlugify,
	rehypeMdxTitle,
	rehypeTableOfContents,
	rehypeDescription,
];
