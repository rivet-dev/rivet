import { transformerNotationHighlight } from "@shikijs/transformers";
import { useEffect, useMemo, useState } from "react";
import {
	createHighlighterCore,
	createOnigurumaEngine,
	type HighlighterCore,
	type ThemeInput,
} from "shiki";
import { Skeleton } from "../ui/skeleton";
import theme from "./theme.json";

const langs = {
	typescript: () => import("@shikijs/langs/typescript"),
	json: () => import("@shikijs/langs/json"),
	bash: () => import("@shikijs/langs/bash"),
	markdown: () => import("@shikijs/langs/markdown"),
};

let highlighterPromise: Promise<HighlighterCore> | null = null;
let highlighterInstance: HighlighterCore | null = null;

async function getHighlighter(
	language: keyof typeof langs,
): Promise<HighlighterCore> {
	if (highlighterInstance !== null) {
		const loadedLangs = highlighterInstance.getLoadedLanguages();
		if (!loadedLangs.includes(language)) {
			await highlighterInstance.loadLanguage(await langs[language]());
		}
		return highlighterInstance;
	}

	if (highlighterPromise === null) {
		highlighterPromise = langs[language]()
			.then((langModule) =>
				createHighlighterCore({
					themes: [theme as ThemeInput],
					langs: [langModule],
					engine: createOnigurumaEngine(import("shiki/wasm")),
				}),
			)
			.then((hl) => {
				highlighterInstance = hl;
				return hl;
			});
	}

	const hl = await highlighterPromise;
	const loadedLangs = hl.getLoadedLanguages();
	if (!loadedLangs.includes(language)) {
		await hl.loadLanguage(await langs[language]());
	}
	return hl;
}

interface CodePreviewProps {
	code: string;
	language: keyof typeof langs;
	className?: string;
}

export function CodePreview({ className, code, language }: CodePreviewProps) {
	const [isLoading, setIsLoading] = useState(true);
	const [highlighter, setHighlighter] = useState<HighlighterCore | null>(
		null,
	);

	useEffect(() => {
		let cancelled = false;
		void getHighlighter(language).then((hl) => {
			if (!cancelled) {
				setHighlighter(hl);
				setIsLoading(false);
			}
		});
		return () => {
			cancelled = true;
		};
	}, [language]);

	const result = useMemo(
		() =>
			isLoading || !highlighter
				? ""
				: (highlighter.codeToHtml(code, {
						lang: language,
						theme: theme.name,
						transformers: [transformerNotationHighlight()],
					}) as TrustedHTML),
		[isLoading, highlighter, code, language],
	);

	if (isLoading) {
		return (
			<div className="px-2 flex flex-col gap-0.5">
				<Skeleton className="w-full h-5" />
				<Skeleton className="w-full h-5" />
				<Skeleton className="w-full h-5" />
				<Skeleton className="w-full h-5" />
				<Skeleton className="w-full h-5" />
			</div>
		);
	}

	return (
		<div
			className={className}
			// biome-ignore lint/security/noDangerouslySetInnerHtml: its safe
			dangerouslySetInnerHTML={{ __html: result }}
		/>
	);
}
