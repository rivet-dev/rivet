import { transformerNotationFocus } from "@shikijs/transformers";
import { slugifyWithCounter } from "@sindresorhus/slugify";
import * as acorn from "acorn";
import { toString as mdastToString } from "mdast-util-to-string";
import { mdxAnnotations } from "mdx-annotations";
import rehypeMdxTitle from "rehype-mdx-title";
import * as shiki from "shiki";
import { visit } from "unist-util-visit";

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
				if (parentNode.properties?.annotation) {
					try {
						// Annotations can only be strings
						const annotations = JSON.parse(
							parentNode.properties.annotation,
						);

						for (const key in annotations) {
							parentNode.properties[key] = annotations[key];
						}
					} catch (e) {
						console.error(parentNode.properties.annotation);
						console.error("invalid annotations in code block", e);
					}
				}
			}
		});
	};
}

const cssVariableTheme = shiki.createCssVariablesTheme({
	name: "css-variables",
	variablePrefix: "--shiki-",
	variableDefaults: {},
	fontStyle: true,
});

/** @type {import("shiki").Highlighter} */
let highlighter;

function rehypeShiki() {
	return async (tree) => {
		highlighter ??= await shiki.getSingletonHighlighter({
			theme: cssVariableTheme,
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
			],
		});

		visit(tree, "element", (node) => {
			if (
				node.tagName === "pre" &&
				node.children[0]?.tagName === "code"
			) {
				const codeNode = node.children[0];
				const textNode = codeNode.children[0];

				node.properties.code = textNode.value;

				if (node.properties.language) {
					textNode.value = highlighter.codeToHtml(textNode.value, {
						lang: node.properties.language,
						theme: cssVariableTheme,
						transformers: [transformerNotationFocus()],
					});
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
