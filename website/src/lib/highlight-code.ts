import * as shiki from "shiki";
import heroTheme from "@/lib/agent-os-hero-code-theme";

const LANGS: shiki.BundledLanguage[] = [
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
	"powershell",
	"ts",
	"typescript",
	"yaml",
	"http",
	"prisma",
	"rust",
	"swift",
	"toml",
];

let highlighter: shiki.Highlighter;

export async function highlightCodeHtml(
	code: string,
	lang: shiki.BundledLanguage | string = "ts",
) {
	highlighter ??= await shiki.getSingletonHighlighter({
		langs: LANGS,
		themes: [heroTheme],
	});

	return highlighter.codeToHtml(code, {
		lang: (lang as shiki.BundledLanguage) || "text",
		theme: heroTheme.name,
	});
}
