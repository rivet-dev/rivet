import { mkdirSync, writeFileSync, existsSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import * as readline from "node:readline";

const __dirname = dirname(fileURLToPath(import.meta.url));
const websiteDir = resolve(__dirname, "..");
const postsDir = resolve(websiteDir, "src/content/posts");

const AUTHORS = ["nathan-flurry", "nicholas-kissel", "forest-anderson"] as const;
const CATEGORIES = ["changelog", "monthly-update", "launch-week", "technical", "guide", "frogs"] as const;

type Author = (typeof AUTHORS)[number];
type Category = (typeof CATEGORIES)[number];

function createSlug(title: string): string {
	return title
		.toLowerCase()
		.replace(/[^a-z0-9\s-]/g, "")
		.replace(/\s+/g, "-")
		.replace(/-+/g, "-")
		.trim();
}

function getDateString(): string {
	const now = new Date();
	const year = now.getFullYear();
	const month = String(now.getMonth() + 1).padStart(2, "0");
	const day = String(now.getDate()).padStart(2, "0");
	return `${year}-${month}-${day}`;
}

function prompt(rl: readline.Interface, question: string): Promise<string> {
	return new Promise((resolve) => {
		rl.question(question, (answer) => {
			resolve(answer.trim());
		});
	});
}

async function selectOption<T extends string>(
	rl: readline.Interface,
	question: string,
	options: readonly T[]
): Promise<T> {
	console.log(`\n${question}`);
	options.forEach((opt, i) => console.log(`  ${i + 1}. ${opt}`));

	while (true) {
		const answer = await prompt(rl, `Select (1-${options.length}): `);
		const index = parseInt(answer, 10) - 1;
		if (index >= 0 && index < options.length) {
			return options[index];
		}
		console.log("Invalid selection, please try again.");
	}
}

async function main() {
	const rl = readline.createInterface({
		input: process.stdin,
		output: process.stdout,
	});

	console.log("\n📝 Create a new blog post\n");

	const title = await prompt(rl, "Title: ");
	if (!title) {
		console.error("Title is required");
		rl.close();
		process.exit(1);
	}

	const description = await prompt(rl, "Description: ");
	if (!description) {
		console.error("Description is required");
		rl.close();
		process.exit(1);
	}

	const author = await selectOption(rl, "Author:", AUTHORS);
	const category = await selectOption(rl, "Category:", CATEGORIES);

	const keywordsInput = await prompt(rl, "Keywords (comma-separated, optional): ");
	const keywords = keywordsInput
		? keywordsInput
				.split(",")
				.map((k) => k.trim())
				.filter(Boolean)
		: [];

	rl.close();

	const dateStr = getDateString();
	const slug = createSlug(title);
	const folderName = `${dateStr}-${slug}`;
	const postDir = resolve(postsDir, folderName);

	if (existsSync(postDir)) {
		console.error(`\nError: Post directory already exists: ${postDir}`);
		process.exit(1);
	}

	const keywordsLine = keywords.length > 0 ? `keywords: ${JSON.stringify(keywords)}\n` : "";

	const frontmatter = `---
author: ${author}
published: "${dateStr}"
category: ${category}
${keywordsLine}title: "${title}"
description: "${description}"
---

`;

	mkdirSync(postDir, { recursive: true });
	writeFileSync(resolve(postDir, "page.mdx"), frontmatter);

	console.log(`\n✅ Created new post at: ${postDir}/page.mdx`);
}

main();
