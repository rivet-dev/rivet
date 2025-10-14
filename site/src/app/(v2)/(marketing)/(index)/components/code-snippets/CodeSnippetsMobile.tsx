"use client";

import { useState, useEffect } from "react";
import {
	Icon,
	faGithub,
	faChevronDown,
	faChevronRight,
	faCode,
} from "@rivet-gg/icons";
import {
	examples,
	type ExampleData,
} from "@/data/examples/examples";
import { EXAMPLE_ICON_MAP, createExampleActions } from "./utils";
import * as shiki from "shiki";
import theme from "@/lib/textmate-code-theme";

interface ExampleListItemProps {
	example: ExampleData;
	icon: any;
	isExpanded: boolean;
	onToggle: () => void;
}

let highlighter: shiki.Highlighter;

function ExampleListItem({ example, icon, isExpanded, onToggle }: ExampleListItemProps) {
	const [fileContent, setFileContent] = useState<string>("");
	const [isCodeExpanded, setIsCodeExpanded] = useState<boolean>(false);
	const { handleOpenGithub } = createExampleActions(example.id, example.files);

	// Get the main file to display
	const mainFile = example.filesToOpen[0] || Object.keys(example.files)[0];

	// Reset code expanded state when accordion is collapsed
	useEffect(() => {
		if (!isExpanded) {
			setIsCodeExpanded(false);
		}
	}, [isExpanded]);

	// Initialize highlighter and highlight code when expanded
	useEffect(() => {
		const highlightCode = async () => {
			if (!isExpanded || !mainFile) return;

			highlighter ??= await shiki.getSingletonHighlighter({
				langs: ["typescript", "json"],
				themes: [theme],
			});

			const code = example.files[mainFile] || "";
			const lang = mainFile.endsWith(".json") ? "json" : "typescript";

			const highlighted = highlighter.codeToHtml(code, {
				lang,
				theme: theme.name,
			});

			setFileContent(highlighted);
		};

		highlightCode();
	}, [isExpanded, mainFile, example.files]);

	return (
		<div className="border border-white/15 rounded-lg overflow-hidden bg-white/[0.06]">
			<button
				onClick={onToggle}
				className="w-full p-3 flex items-center gap-2.5 text-left hover:bg-white/[0.04] transition-colors"
			>
				<Icon
					icon={isExpanded ? faChevronDown : faChevronRight}
					className="w-3 h-3 text-white/50 flex-shrink-0"
				/>
				<Icon icon={icon} className="w-4 h-4 text-white/70 flex-shrink-0" />
				<div className="flex-1 min-w-0">
					<h3 className="text-white font-medium text-sm">{example.title}</h3>
				</div>
			</button>

			{isExpanded && (
				<div className="border-t border-white/15">
					{/* Code snippet */}
					<div className="bg-[#0d0b0a] relative">
						<div className="relative">
							<div
								className={`code p-3 text-xs overflow-x-auto overflow-y-hidden transition-all duration-300 ${
									isCodeExpanded ? "" : "max-h-[900px]"
								}`}
								// biome-ignore lint/security/noDangerouslySetInnerHtml: we trust shiki
								dangerouslySetInnerHTML={{ __html: fileContent }}
							/>

							{/* Gradient overlay and Show More button */}
							{!isCodeExpanded && (
								<div className="absolute bottom-0 left-0 right-0 h-24 bg-gradient-to-t from-[#0d0b0a] via-[#0d0b0a]/90 to-transparent flex items-end justify-center pb-3">
									<button
										onClick={() => setIsCodeExpanded(true)}
										className="px-4 py-1.5 text-xs font-medium text-white/80 hover:text-white bg-white/10 hover:bg-white/15 border border-white/20 hover:border-white/30 rounded-md transition-all duration-200"
									>
										Show more
									</button>
								</div>
							)}
						</div>
					</div>

					{/* GitHub button - only shown when expanded */}
					<div className="p-3 border-t border-white/15 bg-white/[0.02]">
						<button
							onClick={handleOpenGithub}
							className="w-full flex items-center justify-center gap-2 px-3 py-2 text-xs font-medium text-white/80 hover:text-white hover:bg-white/8 border border-white/15 hover:border-white/25 rounded-md transition-all duration-200"
						>
							<Icon icon={faGithub} className="w-3.5 h-3.5" />
							View on GitHub
						</button>
					</div>
				</div>
			)}
		</div>
	);
}

export default function CodeSnippetsMobile() {
	const [expandedExamples, setExpandedExamples] = useState<Set<string>>(new Set());

	const toggleExample = (exampleId: string) => {
		setExpandedExamples(prev => {
			const next = new Set(prev);
			if (next.has(exampleId)) {
				next.delete(exampleId);
			} else {
				next.add(exampleId);
			}
			return next;
		});
	};

	const examplesWithIcons = examples.map((example) => ({
		...example,
		icon: EXAMPLE_ICON_MAP[example.id] || faCode,
	}));

	return (
		<div>
			<h2 className="text-center text-white/70 text-sm font-medium mb-4">
				Examples
			</h2>
			<div className="space-y-3">
				{examplesWithIcons.map((example) => (
					<ExampleListItem
						key={example.id}
						example={example}
						icon={example.icon}
						isExpanded={expandedExamples.has(example.id)}
						onToggle={() => toggleExample(example.id)}
					/>
				))}
			</div>
		</div>
	);
}
