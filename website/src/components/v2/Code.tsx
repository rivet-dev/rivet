import {
	Button,
	cn,
	ScrollArea,
	TooltipProvider,
	WithTooltip,
} from "@rivet-gg/components";
import {
	faCode,
	faCopy,
	faDatabase,
	faDocker,
	faFile,
	faGear,
	faGolang,
	faJs,
	faPhp,
	faPython,
	faRust,
	faSwift,
	faTerminal,
	faTypescript,
	Icon,
} from "@rivet-gg/icons";
import type { IconDefinition } from "@fortawesome/fontawesome-svg-core";
import escapeHTML from "escape-html";
import { cloneElement, type ReactElement, type ReactNode } from "react";
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

const languageIcons: Record<string, IconDefinition> = {
	js: faJs,
	ts: faTypescript,
	typescript: faTypescript,
	python: faPython,
	php: faPhp,
	rust: faRust,
	go: faGolang,
	docker: faDocker,
	dockerfile: faDocker,
	swift: faSwift,
	bash: faTerminal,
	sh: faTerminal,
	ps1: faTerminal,
	powershell: faTerminal,
	sql: faDatabase,
	ini: faGear,
};

interface CodeGroupProps {
	className?: string;
	children: ReactNode;
	workspace?: boolean;
}

export function CodeGroup({ children, className, workspace }: CodeGroupProps) {
	if (workspace) {
		return (
			<div
				className={cn("code-group group my-4 overflow-hidden rounded-xl", className)}
				data-code-group-container
				data-code-group-workspace
			>
				<div className="flex min-h-[200px] gap-2">
					<div
						data-code-group-sidebar
						className="flex flex-col w-[160px] shrink-0 py-2 overflow-y-auto"
					>
						{/* File tree items populated by TabsScript.astro */}
					</div>
					<div data-code-group-content-container className="flex-1 min-w-0">
						{/* Content is moved here by TabsScript.astro */}
					</div>
				</div>
				<div data-code-group-source className="hidden">
					{children}
				</div>
			</div>
		);
	}

	// Use Tabs-like pattern: render container with hidden source, let TabsScript create tabs
	return (
		<div
			className={cn("code-group group my-4 overflow-hidden rounded-xl border bg-neutral-950", className)}
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
			<div data-code-group-content-container>
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
	code?: string;
	highlightedCode?: string;
	flush?: boolean;
	hide?: boolean;
	className?: string | string[];
}

function looksLikeMermaid(code: string): boolean {
	const firstLine = code
		.split("\n")
		.map((line) => line.trim())
		.find((line) => line.length > 0);
	if (!firstLine) return false;

	return /^(sequenceDiagram|flowchart|graph|classDiagram|stateDiagram|erDiagram|journey|gantt|pie|mindmap|timeline|gitGraph|quadrantChart|requirementDiagram|C4Context|C4Container|C4Component|C4Dynamic|C4Deployment)\b/.test(
		firstLine,
	);
}

function parseCodeMeta(meta: string | undefined) {
	if (!meta) {
		return { title: undefined as string | undefined, hide: false, nocheck: false };
	}

	let parsedTitle: string | undefined;
	let parsedHide = false;
	let parsedNocheck = false;

	for (const token of meta.trim().split(/\s+/)) {
		if (token === "@hide") {
			parsedHide = true;
		} else if (token === "@nocheck") {
			parsedNocheck = true;
		} else if (token && !token.startsWith("@")) {
			parsedTitle = token;
		}
	}

	return { title: parsedTitle, hide: parsedHide, nocheck: parsedNocheck };
}

function normalizeClassNames(value: unknown): string[] {
	if (typeof value === "string") {
		return value.split(/\s+/).filter(Boolean);
	}
	if (Array.isArray(value)) {
		return value.flatMap((entry) => normalizeClassNames(entry));
	}
	return [];
}

export const pre = ({
	children,
	file,
	language,
	title,
	isInGroup,
	code,
	highlightedCode,
	flush,
	hide,
	className,
}: PreProps) => {
	const codeChild =
		children && typeof children === "object" && "props" in children
			? (children as ReactElement<{
				metastring?: string;
				meta?: string;
				annotation?: string;
				className?: string | string[];
			  }>).props
			: undefined;
	const parsedMeta = parseCodeMeta(
		codeChild?.metastring ?? codeChild?.meta ?? codeChild?.annotation,
	);
	const preClassNames = normalizeClassNames(className);
	const codeClassNames = normalizeClassNames(codeChild?.className);
	const sourceText =
		typeof code === "string" ? code : (extractTextContent(children) ?? "");
	const isMermaid =
		language === "mermaid" ||
		preClassNames.includes("mermaid") ||
		codeClassNames.includes("mermaid") ||
		codeClassNames.includes("language-mermaid") ||
		looksLikeMermaid(sourceText);
	if (isMermaid) {
		return <pre className="mermaid">{sourceText}</pre>;
	}
	const resolvedTitle = title ?? parsedMeta.title;
	const resolvedHide = hide ?? parsedMeta.hide;

	// Calculate display name for tabs
	const displayName =
		resolvedTitle || languageNames[language as keyof typeof languageNames] || "Code";
	// Calculate unique identifier for tab matching
	const tabId = file || resolvedTitle || language || "text";

	const langIcon = file ? faFile : (languageIcons[language as string] ?? faCode);

	const codeBlock = (
		<div
			className={cn(
				"not-prose group/code relative group-[.code-group]:my-0 group-[.code-group]:border-none group-[.code-group]:overflow-visible",
				flush ? "" : "my-4 overflow-hidden rounded-xl border"
			)}
			data-code-block
			data-code-title={displayName}
			data-code-id={tabId}
			data-code-hide={resolvedHide ? "true" : undefined}
			data-code-lang={language || undefined}
		>
			<span data-code-icon className="hidden">
				<Icon icon={langIcon} className="size-3" />
			</span>
			<div className="absolute right-2 top-2 z-10 opacity-0 transition-opacity group-hover/code:opacity-100">
				<TooltipProvider>
					<WithTooltip
						trigger={
							<CopyCodeTrigger>
								<Button size="icon-sm" variant="ghost" data-copy-code className="hover:bg-neutral-700/80">
									<Icon icon={faCopy} />
								</Button>
							</CopyCodeTrigger>
						}
						content="Copy code"
					/>
				</TooltipProvider>
			</div>

			<div className="bg-neutral-950 text-sm overflow-x-auto">
				<div className="p-4 w-fit min-w-full">
					{highlightedCode ? (
						<span
							className="not-prose code [&_pre]:!bg-transparent [&_pre]:!m-0 [&_pre]:!p-0"
							// biome-ignore lint/security/noDangerouslySetInnerHtml: it's generated from shiki
							dangerouslySetInnerHTML={{ __html: highlightedCode }}
						/>
					) : code ? (
						<pre className="not-prose code whitespace-pre">{code}</pre>
					) : children ? (
						cloneElement(children, { escaped: true })
					) : null}
				</div>
			</div>
		</div>
	);

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
