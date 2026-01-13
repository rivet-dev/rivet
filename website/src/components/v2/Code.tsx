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
import {
	Children,
	cloneElement,
	isValidElement,
	type ReactElement,
} from "react";
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
	children: ReactElement[];
}

const getChildIdx = (child: ReactElement) =>
	child.props?.file || child.props?.title || child.props?.language || "text";

const getDisplayName = (child: ReactElement) =>
	child.props?.title || languageNames[child.props?.language] || "Code";

export function CodeGroup({ children, className }: CodeGroupProps) {
	const tabChildren = Children.toArray(children).filter(
		(child): child is ReactElement => isValidElement(child),
	);

	if (tabChildren.length === 0) {
		return null;
	}

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
					{tabChildren.map((child, index) => {
						const idx = getChildIdx(child);
						const displayName = getDisplayName(child);
						return (
							<button
								key={idx}
								type="button"
								data-code-group-trigger={idx}
								className={cn(
									"relative inline-flex min-h-[2.75rem] items-center justify-center whitespace-nowrap",
									"rounded-none border-b-2 bg-transparent px-4 py-2.5 text-sm font-semibold",
									"ring-offset-background transition-none",
									"focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
									"disabled:pointer-events-none disabled:opacity-50",
									index === 0
										? "border-b-primary text-white"
										: "border-b-transparent text-muted-foreground",
								)}
							>
								{displayName}
							</button>
						);
					})}
				</div>
			</div>
			<div data-code-group-content-container className="pt-2">
				{tabChildren.map((child, index) => {
					const idx = getChildIdx(child);
					return (
						<div
							key={idx}
							data-code-group-content={idx}
							className={index === 0 ? "" : "hidden"}
						>
							{cloneElement(child, {
								isInGroup: true,
								...child.props,
							})}
						</div>
					);
				})}
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
	const codeBlock = (
		<div className={cn(
			"not-prose group-[.code-group]:my-0 group-[.code-group]:-mt-2 group-[.code-group]:border-none group-[.code-group]:overflow-visible",
			flush ? "" : "my-4 overflow-hidden rounded-md border"
		)}>
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
