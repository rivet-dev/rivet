import * as shiki from "shiki";
import theme from "@/lib/textmate-code-theme";

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
	"json",
	"powershell",
	"ts",
	"typescript",
	"yaml",
	"http",
	"prisma",
	"rust",
];

let highlighter: shiki.Highlighter;

export async function CodeBlock({
	lang,
	code,
}: { lang: shiki.BundledLanguage; code: string }) {
	highlighter ??= await shiki.getSingletonHighlighter({
		langs: LANGS,
		themes: [theme],
	});

	const out = highlighter.codeToHtml(code, {
		lang,
		theme: theme.name,
	});

	return (
		<div
			className="code"
			// biome-ignore lint/security/noDangerouslySetInnerHtml: we trust shinki
			dangerouslySetInnerHTML={{ __html: out }}
		/>
	);
}
