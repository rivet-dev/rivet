import { access, mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { chromium } from "playwright";
import {
	buildTechnicalHtml,
	captureHtml,
	type TechnicalLayout,
} from "./render-launch-images";

interface Options {
	title: string;
	technicalSnippets: string;
	output: string;
	layout: TechnicalLayout;
	browser?: string;
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

	const title = values.get("title");
	const technicalSnippets = values.get("technical-snippets");
	const output = values.get("output");
	if (!title || !technicalSnippets || !output) {
		throw new Error(
			"Usage: pnpm --dir website render-technical-image -- --title <text> --technical-snippets <json-path> --output <png-path> [--layout grid|vertical] [--browser <path>]",
		);
	}

	const layout = values.get("layout") ?? "grid";
	if (layout !== "grid" && layout !== "vertical") {
		throw new Error("--layout must be either grid or vertical");
	}
	if (path.extname(output).toLowerCase() !== ".png") {
		throw new Error("--output must end in .png");
	}

	return {
		title,
		technicalSnippets: path.resolve(technicalSnippets),
		output: path.resolve(output),
		layout,
		browser: values.get("browser")
			? path.resolve(values.get("browser")!)
			: undefined,
	};
}

async function launchBrowser(browserPath?: string) {
	if (browserPath) {
		return chromium.launch({ headless: true, executablePath: browserPath });
	}

	try {
		return await chromium.launch({ headless: true });
	} catch (error) {
		for (const candidate of [
			"/usr/bin/chromium",
			"/usr/bin/chromium-browser",
			"/usr/bin/google-chrome",
		]) {
			try {
				await access(candidate);
				process.stderr.write(
					`Bundled Chromium unavailable; using ${candidate}\n`,
				);
				return chromium.launch({
					headless: true,
					executablePath: candidate,
				});
			} catch {
				// Try the next system browser.
			}
		}
		throw error;
	}
}

async function main(): Promise<void> {
	const options = parseArgs(process.argv.slice(2));
	await mkdir(path.dirname(options.output), { recursive: true });

	const html = await buildTechnicalHtml({
		title: options.title,
		technicalSnippets: options.technicalSnippets,
		technicalLayout: options.layout,
	});
	if (!html) throw new Error("Technical snippets are required");

	const htmlPath = options.output.replace(/\.png$/i, ".html");
	await writeFile(htmlPath, html);

	const browser = await launchBrowser(options.browser);
	try {
		await captureHtml(browser, html, options.output, ".sections", true);
	} finally {
		await browser.close();
	}

	process.stdout.write(
		`${JSON.stringify({ imagePath: options.output, htmlPath }, null, 2)}\n`,
	);
}

main().catch((error: unknown) => {
	console.error(error);
	process.exitCode = 1;
});
