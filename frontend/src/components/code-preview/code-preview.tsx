import { useEffect, useMemo, useRef, useState } from "react";
import {
	createHighlighterCore,
	createOnigurumaEngine,
	type HighlighterCore,
	type ThemeInput,
} from "shiki";
import { Skeleton } from "../ui/skeleton";
import theme from "./theme.json";

interface CodePreviewProps {
	code: string;
	language: "typescript";
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
				langs: [import("@shikijs/langs/typescript")],
				engine: createOnigurumaEngine(import("shiki/wasm")),
			});
		}

		createHighlighter().then(() => {
			setIsLoading(false);
		});

		return () => {
			highlighter.current?.dispose();
		};
	}, []);

	const result = useMemo(
		() =>
			isLoading
				? ""
				: (highlighter.current?.codeToHtml(code, {
						lang: language,
						theme: theme.name,
					}) as TrustedHTML),
		[isLoading, code, language],
	);

	if (isLoading) {
		return <Skeleton className="w-full h-5" />;
	}

	return (
		<div
			className={className}
			// biome-ignore lint/security/noDangerouslySetInnerHtml: its safe
			dangerouslySetInnerHTML={{ __html: result }}
		/>
	);
}
