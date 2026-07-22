import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { chromium } from "playwright";
import sharp from "sharp";
import { getSingletonHighlighter, type BundledLanguage } from "shiki";
import theme from "../../../../website/src/lib/textmate-code-theme";

const BLOG_WIDTH = 2048;
const BLOG_HEIGHT = 1024;
const SOCIAL_WIDTH = 2048;
const SOCIAL_HEIGHT = 1238;
const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(SCRIPT_DIR, "../../../..");
const WEBSITE_DIR = path.join(REPO_ROOT, "website");

interface Options {
	painting: string;
	title: string;
	outputDir: string;
	focalX: number;
	focalY: number;
	technicalSnippets?: string;
}

export type TechnicalLayout = "grid" | "vertical";

export interface TechnicalHtmlOptions {
	title: string;
	technicalSnippets?: string;
	technicalLayout?: TechnicalLayout;
}

interface CodeSection {
	title: string;
	language: BundledLanguage;
	code: string;
}

function parseArgs(argv: string[]): Options {
	const args = argv[0] === "--" ? argv.slice(1) : argv;
	const values = new Map<string, string>();
	for (let index = 0; index < args.length; index += 2) {
		const key = args[index];
		const value = args[index + 1];
		if (!key?.startsWith("--") || value === undefined) {
			throw new Error(`Invalid argument near ${key ?? "end of command"}`);
		}
		values.set(key.slice(2), value);
	}

	const painting = values.get("painting");
	const title = values.get("title");
	const outputDir = values.get("output-dir");
	if (!painting || !title || !outputDir) {
		throw new Error(
			"Usage: pnpm --dir website render-launch-images -- --painting <path> --title <text> --output-dir <path> [--focal-x 0.5] [--focal-y 0.5] [--technical-snippets <json-path>]",
		);
	}

	const focalX = Number(values.get("focal-x") ?? "0.5");
	const focalY = Number(values.get("focal-y") ?? "0.5");
	if (![focalX, focalY].every((value) => value >= 0 && value <= 1)) {
		throw new Error("--focal-x and --focal-y must be between 0 and 1");
	}

	return {
		painting: path.resolve(painting),
		title,
		outputDir: path.resolve(outputDir),
		focalX,
		focalY,
		technicalSnippets: values.get("technical-snippets")
			? path.resolve(values.get("technical-snippets")!)
			: undefined,
	};
}

function escapeHtml(value: string): string {
	return value
		.replaceAll("&", "&amp;")
		.replaceAll("<", "&lt;")
		.replaceAll(">", "&gt;")
		.replaceAll('"', "&quot;")
		.replaceAll("'", "&#039;");
}

function dataUrl(mime: string, bytes: Buffer): string {
	return `data:${mime};base64,${bytes.toString("base64")}`;
}

async function cropPainting(
	options: Options,
	outputPath: string,
): Promise<void> {
	const metadata = await sharp(options.painting).metadata();
	if (!metadata.width || !metadata.height) {
		throw new Error(
			`Could not read painting dimensions: ${options.painting}`,
		);
	}

	const targetRatio = BLOG_WIDTH / BLOG_HEIGHT;
	const sourceRatio = metadata.width / metadata.height;
	let width = metadata.width;
	let height = metadata.height;
	let left = 0;
	let top = 0;

	if (sourceRatio > targetRatio) {
		width = Math.round(metadata.height * targetRatio);
		left = Math.round(
			Math.min(
				metadata.width - width,
				Math.max(0, metadata.width * options.focalX - width / 2),
			),
		);
	} else {
		height = Math.round(metadata.width / targetRatio);
		top = Math.round(
			Math.min(
				metadata.height - height,
				Math.max(0, metadata.height * options.focalY - height / 2),
			),
		);
	}

	await sharp(options.painting)
		.extract({ left, top, width, height })
		.resize(BLOG_WIDTH, BLOG_HEIGHT)
		.png()
		.toFile(outputPath);
}

async function buildSocialHtml(
	options: Options,
	blogImage: Buffer,
): Promise<string> {
	const [font, logo] = await Promise.all([
		readFile(
			path.join(
				WEBSITE_DIR,
				"public/fonts/perfectly-nineties/PerfectlyNineties-Semibold.otf",
			),
		),
		readFile(
			path.join(
				WEBSITE_DIR,
				"src/images/rivet-logos/icon-text-black.svg",
			),
			"utf8",
		),
	]);

	return `<!doctype html>
<html lang="en">
	<head>
		<meta charset="utf-8" />
		<meta name="viewport" content="width=${SOCIAL_WIDTH}, initial-scale=1" />
		<style>
			@font-face {
				font-family: "Perfectly Nineties";
				src: url("${dataUrl("font/otf", font)}") format("opentype");
				font-style: normal;
				font-weight: 600;
			}

			* { box-sizing: border-box; }
			html, body {
				margin: 0;
				width: ${SOCIAL_WIDTH}px;
				height: ${SOCIAL_HEIGHT}px;
				overflow: hidden;
				background: #ebebeb;
			}
			.card {
				position: relative;
				width: ${SOCIAL_WIDTH}px;
				height: ${SOCIAL_HEIGHT}px;
				background: #ebebeb;
			}
			.painting {
				position: absolute;
				left: 132px;
				top: 119px;
				width: 1768px;
				height: 816px;
				object-fit: cover;
			}
			.title {
				position: absolute;
				left: 132px;
				top: 991px;
				margin: 0;
				max-width: 1450px;
				color: #000;
				font-family: "Perfectly Nineties", serif;
				font-size: 100px;
				font-style: normal;
				font-weight: 600;
				line-height: 136px;
				letter-spacing: 0;
				white-space: nowrap;
			}
			.logo {
				position: absolute;
				left: 1702px;
				top: 1036px;
				width: 198px;
				height: 81px;
				display: flex;
				align-items: center;
			}
			.logo svg {
				display: block;
				width: 198px;
				height: auto;
			}
		</style>
	</head>
	<body>
		<main class="card">
			<img class="painting" src="${dataUrl("image/png", blogImage)}" alt="" />
			<h1 class="title">${escapeHtml(options.title)}</h1>
			<div class="logo" aria-label="Rivet">${logo}</div>
		</main>
	</body>
</html>`;
}

async function readCodeSections(inputPath: string): Promise<CodeSection[]> {
	const input = JSON.parse(await readFile(inputPath, "utf8")) as unknown;
	if (!Array.isArray(input) || input.length === 0 || input.length > 4) {
		throw new Error(
			"--technical-snippets must contain a JSON array with one to four sections",
		);
	}

	return input.map((section, index) => {
		if (
			typeof section !== "object" ||
			section === null ||
			!("title" in section) ||
			typeof section.title !== "string" ||
			!("code" in section) ||
			typeof section.code !== "string" ||
			("language" in section && typeof section.language !== "string")
		) {
			throw new Error(`Invalid code section at index ${index}`);
		}

		return {
			title: section.title,
			language: ("language" in section
				? section.language
				: "ts") as BundledLanguage,
			code: section.code,
		};
	});
}

export async function buildTechnicalHtml(
	options: TechnicalHtmlOptions,
): Promise<string | undefined> {
	if (!options.technicalSnippets) return undefined;

	const sections = await readCodeSections(options.technicalSnippets);
	const [monoFont, sansFont, displayFont, logo] = await Promise.all([
		readFile(
			path.join(
				WEBSITE_DIR,
				"public/fonts/jetbrains-mono/JetBrainsMono-Variable-latin.woff2",
			),
		),
		readFile(
			path.join(
				WEBSITE_DIR,
				"public/fonts/manrope/Manrope-Variable-latin.woff2",
			),
		),
		readFile(
			path.join(
				WEBSITE_DIR,
				"public/fonts/perfectly-nineties/PerfectlyNineties-Semibold.otf",
			),
		),
		readFile(
			path.join(
				WEBSITE_DIR,
				"src/images/rivet-logos/icon-text-black.svg",
			),
			"utf8",
		),
	]);
	const highlighter = await getSingletonHighlighter({
		langs: [...new Set(sections.map(({ language }) => language))],
		themes: [theme],
	});
	const renderedSections = sections
		.map(({ title, language, code }) => {
			const highlighted = highlighter.codeToHtml(code, {
				lang: language,
				theme: theme.name,
			});
			return `<section class="section">
				<h2>${escapeHtml(title)}</h2>
				<div class="code-block"><div class="code-inner">${highlighted}</div></div>
			</section>`;
		})
		.join("\n");

	return `<!doctype html>
<html lang="en">
	<head>
		<meta charset="utf-8" />
		<meta name="viewport" content="width=${SOCIAL_WIDTH}, initial-scale=1" />
		<style>
			@font-face {
				font-family: "JetBrains Mono";
				src: url("${dataUrl("font/woff2", monoFont)}") format("woff2");
				font-style: normal;
				font-weight: 100 800;
			}
			@font-face {
				font-family: "Manrope";
				src: url("${dataUrl("font/woff2", sansFont)}") format("woff2");
				font-style: normal;
				font-weight: 200 800;
			}
			@font-face {
				font-family: "Perfectly Nineties";
				src: url("${dataUrl("font/otf", displayFont)}") format("opentype");
				font-style: normal;
				font-weight: 600;
			}

			* { box-sizing: border-box; }
			html, body {
				margin: 0;
				width: ${SOCIAL_WIDTH}px;
				height: auto;
				overflow-x: hidden;
				background: #fff;
			}
			.card {
				position: relative;
				width: ${SOCIAL_WIDTH}px;
				height: auto;
				padding: 84px 132px 132px;
				background: #fff;
			}
			.header {
				display: flex;
				align-items: center;
				justify-content: space-between;
				height: 81px;
			}
			.header h1 {
				margin: 0;
				color: #18181b;
				font-family: "Perfectly Nineties", serif;
				font-size: 88px;
				font-weight: 600;
				line-height: 81px;
				letter-spacing: 0;
			}
			.logo {
				width: 198px;
				height: 81px;
				display: flex;
				align-items: center;
			}
			.logo svg {
				display: block;
				width: 198px;
				height: auto;
			}
			.sections {
				display: grid;
				grid-template-columns: ${
					options.technicalLayout === "vertical"
						? "minmax(0, 1fr)"
						: "repeat(2, minmax(0, 1fr))"
				};
				gap: 34px 36px;
				margin-top: 64px;
			}
			.section:only-child {
				grid-column: 1 / -1;
			}
			.sections-3 .section:first-child {
				grid-column: 1;
				grid-row: 1 / span 2;
			}
			.sections-3 .section:nth-child(2) {
				grid-column: 2;
				grid-row: 1;
			}
			.sections-3 .section:nth-child(3) {
				grid-column: 2;
				grid-row: 2;
			}
			.section {
				display: flex;
				min-width: 0;
				flex-direction: column;
			}
			.section h2 {
				margin: 0 0 16px;
				color: #18181b;
				font-family: "Manrope", sans-serif;
				font-size: 30px;
				font-weight: 600;
				line-height: 42px;
				letter-spacing: -0.01em;
			}
			.code-block {
				flex: 1;
				overflow: hidden;
				border: 2px solid rgba(244, 241, 231, 0.1);
				border-radius: 24px;
				background: #0a0a0a;
			}
			.code-inner {
				padding: 30px 32px;
			}
			.shiki {
				margin: 0;
				padding: 0;
				background: transparent !important;
				font-family: "JetBrains Mono", monospace;
				font-size: 34px;
				font-weight: 400;
				line-height: 48px;
				tab-size: 4;
			}
			.shiki code { display: block; }
		</style>
	</head>
	<body>
		<main class="card">
			<header class="header">
				<h1>${escapeHtml(options.title)}</h1>
				<div class="logo" aria-label="Rivet">${logo}</div>
			</header>
			<div class="sections sections-${sections.length}">${renderedSections}</div>
		</main>
	</body>
</html>`;
}

export async function captureHtml(
	browser: Awaited<ReturnType<typeof chromium.launch>>,
	html: string,
	outputPath: string,
	contentSelector?: string,
	dynamicHeight = false,
): Promise<void> {
	const page = await browser.newPage({
		viewport: { width: SOCIAL_WIDTH, height: SOCIAL_HEIGHT },
		deviceScaleFactor: 1,
	});
	try {
		await page.setContent(html, { waitUntil: "load" });
		await page.evaluate(() => document.fonts.ready);
		if (dynamicHeight) {
			const height = await page.$eval(".card", (card) =>
				Math.ceil(card.getBoundingClientRect().height),
			);
			await page.setViewportSize({ width: SOCIAL_WIDTH, height });
		}
		if (contentSelector) {
			const overflowingBlock = await page.$$eval(
				".code-block",
				(blocks) =>
					blocks
						.map((block, index) => ({
							index,
							overflow: block.scrollWidth - block.clientWidth,
						}))
						.find(({ overflow }) => overflow > 1),
			);
			if (overflowingBlock) {
				throw new Error(
					`Code section ${overflowingBlock.index + 1} overflows horizontally by ${Math.ceil(overflowingBlock.overflow)}px; shorten or wrap the snippet`,
				);
			}
		}
		await page.screenshot({ path: outputPath, fullPage: false });
	} finally {
		await page.close();
	}
}

async function main(): Promise<void> {
	const options = parseArgs(process.argv.slice(2));
	await mkdir(options.outputDir, { recursive: true });

	const blogPath = path.join(options.outputDir, "image.png");
	const socialPath = path.join(options.outputDir, "social.png");
	const htmlPath = path.join(options.outputDir, "social.html");
	const technicalPath = path.join(options.outputDir, "technical.png");
	const technicalHtmlPath = path.join(options.outputDir, "technical.html");

	await cropPainting(options, blogPath);
	const blogImage = await readFile(blogPath);
	const html = await buildSocialHtml(options, blogImage);
	await writeFile(htmlPath, html);
	const technicalHtml = await buildTechnicalHtml(options);
	if (technicalHtml) await writeFile(technicalHtmlPath, technicalHtml);

	const browser = await chromium.launch({ headless: true });
	try {
		await captureHtml(browser, html, socialPath);
		if (technicalHtml) {
			await captureHtml(
				browser,
				technicalHtml,
				technicalPath,
				".sections",
				true,
			);
		}
	} finally {
		await browser.close();
	}

	process.stdout.write(
		`${JSON.stringify(
			{
				blogPath,
				socialPath,
				htmlPath,
				...(technicalHtml ? { technicalPath, technicalHtmlPath } : {}),
			},
			null,
			2,
		)}\n`,
	);
}

if (
	process.argv[1] &&
	import.meta.url === pathToFileURL(process.argv[1]).href
) {
	main().catch((error: unknown) => {
		console.error(error);
		process.exitCode = 1;
	});
}
