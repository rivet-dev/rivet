import * as shiki from "shiki";
import theme from "@/lib/textmate-code-theme";
import { cn } from "@rivet-gg/components";

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
	"sql",
	"yaml",
	"http",
	"prisma",
	"rust",
	"toml",
];

let highlighter: shiki.Highlighter;

export async function CodeBlock({
	lang,
	code,
	className,
}: { lang?: shiki.BundledLanguage | string; code: string; className?: string }) {
	highlighter ??= await shiki.getSingletonHighlighter({
		langs: LANGS,
		themes: [theme],
	});

	const out = highlighter.codeToHtml(code, {
		lang: (lang as shiki.BundledLanguage) || "text",
		theme: theme.name,
	});

	return (
		<div
			className={cn("code", className)}
			// biome-ignore lint/security/noDangerouslySetInnerHtml: we trust shinki
			dangerouslySetInnerHTML={{ __html: out }}
		/>
	);
}
