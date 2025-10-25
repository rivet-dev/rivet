// import apiPages from '../src/generated/apiPages.json' assert { type: 'json' };
// import engineStyles from '../src/lib/engineStyles.json' assert { type: 'json' };
import { slugifyWithCounter } from "@sindresorhus/slugify";
import glob from "fast-glob";
import { readFile, writeFile } from "node:fs/promises";
import { toString } from "mdast-util-to-string";
import { remark } from "remark";
import { visit } from "unist-util-visit";

export async function generateNavigation() {
	// Process all pages
	const pages = {};
	const mdxFileNames = await glob(
		["src/app/(legacy)/blog/**/*.mdx", "src/content/**/*.mdx"],
		{
			cwd: ".",
		},
	);
	for (const filename of mdxFileNames) {
		const href =
			"/" +
			filename
				.replace(/\/index\.mdx$/, "")
				.replace(/\.mdx$/, "")
				.replace(/^pages\//, "")
				.replace(/^app\//, "")
				.replace(/\/page$/, "")
				.replace("(guide)/", "")
				.replace("(technical)/", "")
				.replace("(posts)/", "")
				.replace("(legacy)/", "");

		pages[href] = await processPage({ path: filename });
	}

	await writeFile(
		"./src/generated/routes.json",
		JSON.stringify({ pages }, null, 2),
		"utf8",
	);

	console.log(`Generated ${Object.keys(pages).length} pages`);
}

async function processPage({ path }) {
	const md = await readFile(path);

	const ast = remark().parse(md);

	// Title
	const firstHeadingIndex = ast.children.findIndex(
		(node) => node.type === "heading",
	);
	const firstHeading = ast.children[firstHeadingIndex];
	let title = "";
	if (firstHeading) {
		title = firstHeading.children[0].value;
	}

	// Description
	let description = null;
	if (firstHeadingIndex !== -1) {
		for (let i = firstHeadingIndex + 1; i < ast.children.length; i++) {
			const node = ast.children[i];
			if (node.type === "paragraph") {
				// Stop iterating once we reach a paragraph. Means there's a description.
				description = node.children[0].value;
				break;
			} else if (node.type === "heading") {
				// Stop iterating once we reach a new heading. Means there's no description.
				break;
			}
		}
	}

	return {
		title,
		description,
	};
}

generateNavigation();
