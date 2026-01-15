import {
	Badge,
	Button,
	cn,
	ScrollArea,
	TooltipProvider,
	WithTooltip,
} from "@rivet-gg/components";
import { faCopy, faFile, Icon } from "@rivet-gg/icons";
import escapeHTML from "escape-html";
import { cloneElement, type ReactElement, type ReactNode } from "react";
import { AutofillCodeBlock } from "@/components/v2/AutofillCodeBlock";
import { AutofillFooter } from "@/components/v2/AutofillFooter";
import { CopyCodeTrigger } from "@/components/v2/CopyCodeButton";

const languageNames: Record<string, string> = {
	csharp: "C#",
	cpp: "C++",
	go: "Go",
	js: "JavaScript",
	json: "JSON",
	php: "PHP",
	python: "Python",
	ruby: "Ruby",
	ts: "TypeScript",
	typescript: "TypeScript",
	sql: "SQL",
	yaml: "YAML",
	gdscript: "GDScript",
	powershell: "Command Line",
	dockerfile: "Dockerfile",
	ini: "Configuration",
	ps1: "Command Line",
	docker: "Docker",
	http: "HTTP",
	bash: "Command Line",
	sh: "Command Line",
	prisma: "Prisma",
	rust: "Rust",
};

interface CodeGroupProps {
	className?: string;
	children: ReactNode;
}

export function CodeGroup({ children, className }: CodeGroupProps) {
	// Use Tabs-like pattern: render container with hidden source, let TabsScript create tabs
	return (
		<div
			className={cn("code-group group my-4 overflow-hidden rounded-md border bg-neutral-950", className)}
			data-code-group-container
		>
			<div className="overflow-x-auto">
				<div
					data-code-group-tabs
					className="inline-flex text-muted-foreground border-b border-neutral-800 w-full"
				>
					{/* Tabs are populated by TabsScript.astro from data-code-group-source */}
				</div>
			</div>
			<div data-code-group-content-container className="pt-2">
				{/* Content is moved here by TabsScript.astro */}
			</div>
			<div data-code-group-source className="hidden">
				{children}
			</div>
		</div>
	);
}

interface PreProps {
	file?: string;
	title?: string;
	language?: keyof typeof languageNames | string;
	isInGroup?: boolean;
	children?: ReactElement;
	autofill?: boolean;
	code?: string;
	highlightedCode?: string;
	flush?: boolean;
}
export const pre = ({
	children,
	file,
	language,
	title,
	isInGroup,
	autofill,
	code,
	highlightedCode,
	flush,
}: PreProps) => {
	// Calculate display name for tabs
	const displayName = title || languageNames[language as keyof typeof languageNames] || "Code";
	// Calculate unique identifier for tab matching
	const tabId = file || title || language || "text";

	const codeBlock = (
		<div
			className={cn(
				"not-prose group-[.code-group]:my-0 group-[.code-group]:-mt-2 group-[.code-group]:border-none group-[.code-group]:overflow-visible",
				flush ? "" : "my-4 overflow-hidden rounded-md border"
			)}
			data-code-block
			data-code-title={displayName}
			data-code-id={tabId}
		>
			<div className="bg-neutral-950 text-wrap p-2 text-sm">
				<ScrollArea className="w-full">
					{highlightedCode ? (
						<span
							className="not-prose code [&_pre]:!bg-transparent [&_pre]:!m-0 [&_pre]:!p-0"
							// biome-ignore lint/security/noDangerouslySetInnerHtml: it's generated from shiki
							dangerouslySetInnerHTML={{ __html: highlightedCode }}
						/>
					) : code ? (
						<pre className="not-prose code whitespace-pre-wrap">{code}</pre>
					) : children ? (
						cloneElement(children, { escaped: true })
					) : null}
				</ScrollArea>
			</div>

			<div className="text-foreground flex items-center justify-between gap-2 border-t bg-black p-2 text-xs">
				<div className="text-muted-foreground flex items-center gap-2">
					{file ? (
						<>
							<Icon icon={faFile} className="block" />
							<span>{file}</span>
						</>
					) : isInGroup ? null : (
						(title || languageNames[language]) && language !== "text" ? (
							<Badge variant="outline">
								{title || languageNames[language]}
							</Badge>
						) : null
					)}
					{autofill && <AutofillFooter />}
				</div>
				<TooltipProvider>
					<WithTooltip
						trigger={
							<CopyCodeTrigger>
								<Button size="icon-sm" variant="ghost" data-copy-code>
									<Icon icon={faCopy} />
								</Button>
							</CopyCodeTrigger>
						}
						content="Copy code"
					/>
				</TooltipProvider>
			</div>
		</div>
	);

	// Wrap with autofill component if enabled
	if (autofill && code) {
		return <AutofillCodeBlock code={code}>{codeBlock}</AutofillCodeBlock>;
	}

	return codeBlock;
};

export { pre as Code };

// Helper to extract string content from children (handles Astro MDX quirks)
function extractTextContent(children: unknown): string | null {
	if (typeof children === 'string') {
		return children;
	}
	if (children === null || children === undefined) {
		return '';
	}
	if (Array.isArray(children)) {
		const extracted = children.map(extractTextContent);
		// If any child couldn't be extracted, return null
		if (extracted.some(e => e === null)) return null;
		return extracted.join('');
	}
	// If it's a React element with props.children or props.dangerouslySetInnerHTML
	if (typeof children === 'object' && children !== null) {
		const obj = children as Record<string, unknown>;
		if ('props' in obj && typeof obj.props === 'object' && obj.props !== null) {
			const props = obj.props as Record<string, unknown>;
			if ('dangerouslySetInnerHTML' in props && typeof props.dangerouslySetInnerHTML === 'object') {
				const html = props.dangerouslySetInnerHTML as { __html?: string };
				if (html.__html) return html.__html;
			}
			if ('children' in props) {
				return extractTextContent(props.children);
			}
		}
		// Check for direct __html property
		if ('__html' in obj && typeof obj.__html === 'string') {
			return obj.__html;
		}
	}
	// Return null to indicate extraction failed - will fall back to rendering children directly
	return null;
}

export const code = ({ children, escaped }) => {
	const textContent = extractTextContent(children);

	// If we couldn't extract text content, render children directly
	// This handles Astro MDX's compiled element format
	if (textContent === null) {
		if (escaped) {
			return <span className="not-prose code">{children}</span>;
		}
		return <code>{children}</code>;
	}

	if (escaped) {
		return (
			<span
				className="not-prose code"
				// biome-ignore lint/security/noDangerouslySetInnerHtml: it's generated from markdown
				dangerouslySetInnerHTML={{ __html: textContent }}
			/>
		);
	}
	return (
		<code
			// biome-ignore lint/security/noDangerouslySetInnerHtml: it's generated from markdown
			dangerouslySetInnerHTML={{ __html: escapeHTML(textContent) }}
		/>
	);
};
