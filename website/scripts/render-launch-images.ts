import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import sharp from "sharp";

const BLOG_WIDTH = 2048;
const BLOG_HEIGHT = 1024;
const SOCIAL_WIDTH = 2048;
const SOCIAL_HEIGHT = 1238;

interface Options {
	painting: string;
	title: string;
	outputDir: string;
	focalX: number;
	focalY: number;
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
			"Usage: pnpm --dir website render-launch-images -- --painting <path> --title <text> --output-dir <path> [--focal-x 0.5] [--focal-y 0.5]",
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
	const scriptDir = path.dirname(fileURLToPath(import.meta.url));
	const websiteDir = path.resolve(scriptDir, "..");
	const [font, logo] = await Promise.all([
		readFile(
			path.join(
				websiteDir,
				"public/fonts/perfectly-nineties/PerfectlyNineties-Semibold.otf",
			),
		),
		readFile(
			path.join(websiteDir, "src/images/rivet-logos/icon-text-black.svg"),
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

async function main(): Promise<void> {
	const options = parseArgs(process.argv.slice(2));
	await mkdir(options.outputDir, { recursive: true });

	const blogPath = path.join(options.outputDir, "image.png");
	const socialPath = path.join(options.outputDir, "social.png");
	const htmlPath = path.join(options.outputDir, "social.html");

	await cropPainting(options, blogPath);
	const blogImage = await readFile(blogPath);
	const html = await buildSocialHtml(options, blogImage);
	await writeFile(htmlPath, html);

	const browser = await chromium.launch({ headless: true });
	try {
		const page = await browser.newPage({
			viewport: { width: SOCIAL_WIDTH, height: SOCIAL_HEIGHT },
			deviceScaleFactor: 1,
		});
		await page.setContent(html, { waitUntil: "load" });
		await page.evaluate(() => document.fonts.ready);
		await page.screenshot({ path: socialPath, fullPage: false });
	} finally {
		await browser.close();
	}

	process.stdout.write(
		`${JSON.stringify({ blogPath, socialPath, htmlPath }, null, 2)}\n`,
	);
}

await main();
