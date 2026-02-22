import fs from "node:fs/promises";
import path from "node:path";
import Typesense from "typesense";

interface DocsPage {
	title: string;
	content: string;
	path: string;
	cleanPath: string;
}

async function getAllDocsFiles(dir: string, basePath = ""): Promise<string[]> {
	const files: string[] = [];
	const entries = await fs.readdir(dir, { withFileTypes: true });

	for (const entry of entries) {
		const fullPath = path.join(dir, entry.name);
		const relativePath = path.join(basePath, entry.name);

		if (entry.isDirectory() && entry.name !== "cloud") {
			files.push(...(await getAllDocsFiles(fullPath, relativePath)));
		} else if (entry.isFile() && entry.name.endsWith(".mdx")) {
			files.push(relativePath);
		}
	}

	return files;
}

function extractFrontmatter(content: string): {
	title?: string;
	description?: string;
	content: string;
} {
	const frontmatterMatch = content.match(/^---\n([\s\S]*?)\n---\n([\s\S]*)$/);
	if (!frontmatterMatch) {
		return { content };
	}

	const [, frontmatter, bodyContent] = frontmatterMatch;
	const title = frontmatter.match(/^title:\s*["']?([^"'\n]+)["']?$/m)?.[1];
	const description = frontmatter.match(
		/^description:\s*["']?([^"'\n]+)["']?$/m,
	)?.[1];

	return { title, description, content: bodyContent };
}

function stripContentBeforeFirstH1(content: string, _filePath: string): string {
	const h1Match = content.match(/^#\s+(.+)$/m);

	if (!h1Match) {
		// In Astro, titles come from frontmatter so h1 in body is optional.
		return content;
	}

	const h1Index = h1Match.index!;
	return content.substring(h1Index);
}

function cleanMdxSyntax(content: string): string {
	return content
		.replace(/import\s+.*?from\s+['"][^'"]*['"];?\s*/g, "")
		.replace(/export\s+.*?;?\s*/g, "")
		.replace(/<[^>]*>/g, "")
		.replace(/\{[^}]*\}/g, "")
		.replace(/\n\s*\n\s*\n/g, "\n\n")
		.trim();
}

function createCleanPath(filePath: string): string {
	return filePath
		.replace(/\.mdx$/, "")
		.replace(/\/index$/, "")
		.replace(/\\/g, "/");
}

async function extractContentFromMdx(filePath: string): Promise<DocsPage> {
	const fullPath = path.join(process.cwd(), "src/content/docs", filePath);
	const fileContent = await fs.readFile(fullPath, "utf-8");
	const { title, content } = extractFrontmatter(fileContent);

	const contentFromH1 = stripContentBeforeFirstH1(content, filePath);
	const cleanContent = cleanMdxSyntax(contentFromH1);

	const h1TitleMatch = cleanContent.match(/^#\s+(.+)$/m);
	const finalTitle =
		title ||
		(h1TitleMatch ? h1TitleMatch[1] : path.basename(filePath, ".mdx"));

	return {
		title: finalTitle,
		content: cleanContent,
		path: filePath,
		cleanPath: createCleanPath(filePath),
	};
}

async function ensureDirectoryExists(dirPath: string) {
	try {
		await fs.mkdir(dirPath, { recursive: true });
	} catch (error) {
		// Directory might already exist
	}
}

async function writeMarkdownFile(page: DocsPage, outputDir: string) {
	const outputPath =
		page.path === "index.mdx"
			? path.join(
					path.dirname(outputDir),
					path.basename(outputDir) + ".md",
				)
			: path.join(outputDir, `${page.cleanPath}.md`);
	const outputDirPath = path.dirname(outputPath);

	await ensureDirectoryExists(outputDirPath);
	await fs.writeFile(outputPath, page.content);
}

async function getAllBlogPosts(): Promise<string[]> {
	const postsDir = path.join(process.cwd(), "src/content/posts");
	const posts: string[] = [];

	try {
		const entries = await fs.readdir(postsDir, { withFileTypes: true });
		for (const entry of entries) {
			if (entry.isDirectory()) {
				const pagePath = path.join(postsDir, entry.name, "page.mdx");
				try {
					await fs.access(pagePath);
					posts.push(entry.name);
				} catch {
					// Skip if page.mdx doesn't exist
				}
			}
		}
	} catch {
		// Posts directory might not exist
	}

	return posts.sort();
}

async function getAllGuides(): Promise<string[]> {
	const guidesDir = path.join(process.cwd(), "src/content/guides");
	const guides: string[] = [];

	try {
		const entries = await fs.readdir(guidesDir, { withFileTypes: true });
		for (const entry of entries) {
			if (entry.isFile() && entry.name.endsWith(".mdx")) {
				const guideName = entry.name.replace(/\.mdx$/, "");
				guides.push(guideName);
			}
		}
	} catch {
		// Guides directory might not exist
	}

	return guides.sort();
}

async function initializeTypesenseClient() {
	const typesenseHost = process.env.TYPESENSE_HOST;
	const typesensePort = process.env.TYPESENSE_PORT || 443;
	const typesenseProtocol = process.env.TYPESENSE_PROTOCOL || "https";
	const typesenseApiKey = process.env.TYPESENSE_API_KEY;
	const typesenseCollectionName =
		process.env.TYPESENSE_COLLECTION_NAME || "rivet-docs";

	if (!typesenseHost || !typesenseApiKey) {
		console.log(
			"Typesense credentials not provided, skipping search indexing",
		);
		return null;
	}

	// Only index on production to avoid PR previews overwriting the search index.
	const railwayEnv = process.env.RAILWAY_ENVIRONMENT;
	if (railwayEnv && railwayEnv !== "production") {
		console.log(
			`Non-production Railway environment (${railwayEnv}), skipping search indexing`,
		);
		return null;
	}

	const client = new Typesense.Client({
		nodes: [
			{
				host: typesenseHost,
				port: typesensePort ? Number(typesensePort) : 8108,
				protocol: typesenseProtocol,
			},
		],
		apiKey: typesenseApiKey,
		connectionTimeoutSeconds: 5,
	});

	return { client, collectionName: typesenseCollectionName };
}

async function setupTypesenseCollection(
	client: Typesense.Client,
	collectionName: string,
) {
	const schema = {
		name: collectionName,
		fields: [
			{ name: "id", type: "string" as const },
			{ name: "title", type: "string" as const },
			{ name: "content", type: "string" as const },
			{ name: "url", type: "string" as const },
			{ name: "hierarchy", type: "object" as const, optional: true },
		],
		enable_nested_fields: true,
	};

	try {
		await client.collections(collectionName).retrieve();
		console.log(`Collection ${collectionName} exists, updating schema...`);
		try {
			await client.collections(collectionName).update(schema);
			console.log(`Updated collection: ${collectionName}`);
		} catch (updateError) {
			console.log(`Failed to update schema, recreating collection...`);
			await client.collections(collectionName).delete();
			await client.collections().create(schema);
			console.log(`Recreated collection: ${collectionName}`);
		}
	} catch (error) {
		console.log(`Collection ${collectionName} doesn't exist, creating...`);
		await client.collections().create(schema);
		console.log(`Created collection: ${collectionName}`);
	}
}

async function indexDocumentsToTypesense(
	client: Typesense.Client,
	collectionName: string,
	pages: DocsPage[],
) {
	if (!client) return;

	const siteUrl = "https://rivet.dev";
	const documents = pages.map((page, index) => ({
		id: `doc_${index}`,
		title: page.title,
		content: page.content,
		url: `${siteUrl}/docs/${page.cleanPath}`,
		hierarchy: {
			lvl0: "Documentation",
			lvl1: page.title,
		},
	}));

	try {
		try {
			await client
				.collections(collectionName)
				.documents()
				.delete({ filter_by: "id:*" });
			console.log(
				`Cleared existing documents from collection: ${collectionName}`,
			);
		} catch (error) {
			console.log(
				"No existing documents to clear or error clearing:",
				error,
			);
		}

		await client.collections(collectionName).documents().import(documents);
		console.log(`Indexed ${documents.length} documents to Typesense`);
	} catch (error) {
		console.error("Error indexing documents to Typesense:", error);
	}
}

async function generateMarkdownAndLlms() {
	const docsDir = path.join(process.cwd(), "src/content/docs");
	const files = await getAllDocsFiles(docsDir);

	const pages: DocsPage[] = [];
	for (const file of files) {
		try {
			const page = await extractContentFromMdx(file);
			pages.push(page);
		} catch (error) {
			console.error(`\nFATAL ERROR: Failed to process ${file}`);
			console.error((error as Error).message);
			process.exit(1);
		}
	}

	pages.sort((a, b) => a.path.localeCompare(b.path));

	const typesenseConfig = await initializeTypesenseClient();

	const markdownOutputDir = path.join(process.cwd(), "public/docs");
	await ensureDirectoryExists(markdownOutputDir);

	console.log(`Generating ${pages.length} individual markdown files...`);
	for (const page of pages) {
		await writeMarkdownFile(page, markdownOutputDir);
	}

	const blogPosts = await getAllBlogPosts();
	const guides = await getAllGuides();

	const siteUrl = "https://rivet.dev";
	const docsUrls = pages
		.map((page) => `${siteUrl}/docs/${page.cleanPath}`)
		.filter((url) => !url.includes("/docs/cloud"));
	const blogUrls = blogPosts.map((post) => `${siteUrl}/blog/${post}`);
	const changelogUrls = blogPosts.map(
		(post) => `${siteUrl}/changelog/${post}`,
	);
	const guideUrls = guides.map((guide) => `${siteUrl}/guides/${guide}`);

	const otherUrls = [
		`${siteUrl}`,
		`${siteUrl}/talk-to-an-engineer`,
		`${siteUrl}/sales`,
		`${siteUrl}/meme/wired-in`,
		`${siteUrl}/pricing`,
		`${siteUrl}/rss/feed.xml`,
		`${siteUrl}/changelog.json`,
		`${siteUrl}/oss-friends`,
		`${siteUrl}/cloud`,
		`${siteUrl}/rivet-vs-cloudflare-workers`,
		`${siteUrl}/changelog`,
		`${siteUrl}/terms`,
		`${siteUrl}/blog`,
		`${siteUrl}/support`,
		`${siteUrl}/acceptable-use`,
		`${siteUrl}/privacy`,
	];

	const allUrls = [
		...otherUrls,
		...docsUrls,
		...blogUrls,
		...changelogUrls,
		...guideUrls,
	].sort();

	const llmsTxtContent = ["# Rivet Documentation Index", "", ...allUrls].join(
		"\n",
	);

	const filteredPagesForLlmsFull = pages.filter(
		(page) => !page.cleanPath.startsWith("cloud"),
	);
	const llmsFullTxtContent = [
		"# Rivet Documentation - Complete",
		"",
		"This file contains the complete documentation for Rivet, an open-source alternative to Durable Objects.",
		"",
		...filteredPagesForLlmsFull.map(
			(page) => `## ${page.title}\n\n${page.content}`,
		),
	].join("\n");

	await fs.writeFile(
		path.join(process.cwd(), "public/llms.txt"),
		llmsTxtContent,
	);
	await fs.writeFile(
		path.join(process.cwd(), "public/llms-full.txt"),
		llmsFullTxtContent,
	);

	if (typesenseConfig) {
		await setupTypesenseCollection(
			typesenseConfig.client,
			typesenseConfig.collectionName,
		);
		await indexDocumentsToTypesense(
			typesenseConfig.client,
			typesenseConfig.collectionName,
			pages,
		);
	}

	console.log(
		`Generated llms.txt with ${allUrls.length} URLs (${pages.length} docs, ${blogPosts.length} blog posts, ${guides.length} guides)`,
	);
	console.log(`Generated llms-full.txt with ${pages.length} pages`);
	console.log(
		`Generated ${pages.length} individual markdown files in public/docs/`,
	);
}

generateMarkdownAndLlms().catch(console.error);
