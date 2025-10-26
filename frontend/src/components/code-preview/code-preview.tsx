import { transformerNotationHighlight } from "@shikijs/transformers";
import { useEffect, useMemo, useRef, useState } from "react";
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
};

interface CodePreviewProps {
	code: string;
	language: keyof typeof langs;
	className?: string;
}

export function CodePreview({ className, code, language }: CodePreviewProps) {
	const [isLoading, setIsLoading] = useState(true);
	const highlighter = useRef<HighlighterCore | null>(null);

	useEffect(() => {
		if (highlighter.current) return;

		async function createHighlighter() {
			highlighter.current ??= await createHighlighterCore({
				themes: [theme as ThemeInput],
				langs: [await langs[language]()],
				engine: createOnigurumaEngine(import("shiki/wasm")),
			});
		}

		createHighlighter().then(() => {
			setIsLoading(false);
		});

		return () => {
			highlighter.current?.dispose();
		};
	}, [language]);

	const result = useMemo(
		() =>
			isLoading
				? ""
				: (highlighter.current?.codeToHtml(code, {
						lang: language,
						theme: theme.name,
						transformers: [transformerNotationHighlight()],
					}) as TrustedHTML),
		[isLoading, code, language],
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
