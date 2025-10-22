"use client";

import {
	Button,
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
	toast,
} from "@rivet-gg/components";
import {
	Icon,
	faChevronDown,
	faCopy,
	faFileText,
	faExternalLink,
	faMessages,
	faMarkdown,
	faArrowUpRight,
	faLink,
} from "@rivet-gg/icons";
import { useState, useEffect } from "react";

interface DocsPageDropdownProps {
	title: string;
	markdownPath: string; // Path to the generated .md file in public/docs/
	currentUrl: string;
}

export function DocsPageDropdown({
	title,
	markdownPath,
	currentUrl,
}: DocsPageDropdownProps) {
	const [isOpen, setIsOpen] = useState(false);
	const [markdownContent, setMarkdownContent] = useState<string>("");

	// Load markdown content when component mounts (still needed for copy functionality)
	useEffect(() => {
		const loadMarkdown = async () => {
			try {
				const response = await fetch(`/${markdownPath}.md`);
				if (response.ok) {
					const content = await response.text();
					setMarkdownContent(content);
				}
			} catch (error) {
				console.error("Failed to load markdown:", error);
			}
		};
		loadMarkdown();
	}, [markdownPath]);

	const copyPage = async () => {
		try {
			await navigator.clipboard.writeText(markdownContent);
			toast.success("Page copied to clipboard");
		} catch (err) {
			console.error("Failed to copy:", err);
			toast.error("Failed to copy page");
		}
	};

	const copyPageAsMarkdown = async () => {
		try {
			await navigator.clipboard.writeText(markdownContent);
			toast.success("Page copied to clipboard");
		} catch (err) {
			console.error("Failed to copy markdown:", err);
			toast.error("Failed to copy page");
		}
	};

	const viewAsMarkdown = () => {
		// Open the static markdown file in a new tab
		window.open(`/${markdownPath}.md`, "_blank");
	};

	const generateAIPrompt = () => {
		const markdownUrl = `${window.location.origin}/${markdownPath}.md`;
		return `Read this document so I can ask questions about it: ${markdownUrl}`;
	};

	const openInChatGPT = () => {
		const prompt = generateAIPrompt();
		const chatGPTUrl = `https://chat.openai.com/?q=${encodeURIComponent(prompt)}`;
		window.open(chatGPTUrl, "_blank");
	};

	const openInClaude = () => {
		const prompt = generateAIPrompt();
		const claudeUrl = `https://claude.ai/new?q=${encodeURIComponent(prompt)}`;
		window.open(claudeUrl, "_blank");
	};

	const copyLinkToMarkdown = async () => {
		try {
			const markdownUrl = `${window.location.origin}/${markdownPath}.md`;
			await navigator.clipboard.writeText(markdownUrl);
			toast.success("Link copied to clipboard");
		} catch (err) {
			console.error("Failed to copy markdown link:", err);
			toast.error("Failed to copy link");
		}
	};

	return (
		<div className="flex items-center">
			<Button
				variant="outline"
				size="sm"
				onClick={copyPage}
				className="h-8 rounded-r-none border-r-0"
			>
				<Icon icon={faMarkdown} className="mr-1 h-4 w-4" />
				Copy Markdown
			</Button>
			<DropdownMenu open={isOpen} onOpenChange={setIsOpen}>
				<DropdownMenuTrigger asChild>
					<Button
						variant="outline"
						size="sm"
						className="h-8 w-8 p-0 rounded-l-none border-l border-l-border/50"
					>
						<Icon icon={faChevronDown} className="h-3 w-3" />
					</Button>
				</DropdownMenuTrigger>
				<DropdownMenuContent align="end" className="w-56 bg-popover">
					<DropdownMenuItem onClick={copyLinkToMarkdown}>
						<Icon icon={faLink} className="mr-2 h-4 w-4" />
						Copy link to Markdown
					</DropdownMenuItem>
					<DropdownMenuItem onClick={viewAsMarkdown}>
						<Icon icon={faMarkdown} className="mr-2 h-4 w-4" />
						<span>View as Markdown</span>
						<Icon
							icon={faArrowUpRight}
							className="opacity-75 ml-auto h-3 w-3"
						/>
					</DropdownMenuItem>
					<DropdownMenuItem onClick={openInChatGPT}>
						<Icon icon={faMessages} className="mr-2 h-4 w-4" />
						<span>Open in ChatGPT</span>
						<Icon
							icon={faArrowUpRight}
							className="opacity-75 ml-auto h-3 w-3"
						/>
					</DropdownMenuItem>
					<DropdownMenuItem onClick={openInClaude}>
						<Icon icon={faMessages} className="mr-2 h-4 w-4" />
						<span>Open in Claude</span>
						<Icon
							icon={faArrowUpRight}
							className="opacity-75 ml-auto h-3 w-3"
						/>
					</DropdownMenuItem>
				</DropdownMenuContent>
			</DropdownMenu>
		</div>
	);
}
