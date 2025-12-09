import {
	Badge,
	Button,
	cn,
	ScrollArea,
	Tabs,
	TabsContent,
	TabsList,
	TabsTrigger,
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

const languageNames = {
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
			data-code-group
		>
			<Tabs defaultValue={getChildIdx(tabChildren[0])}>
				<ScrollArea
					className="w-full"
					viewportProps={{ className: "[&>div]:!table" }}
				>
					<TabsList>
						{tabChildren.map((child) => {
							const idx = getChildIdx(child);
							const displayName = child.props.title || languageNames[child.props.language];
							return (
								<TabsTrigger key={idx} value={idx}>
									{displayName || "Code"}
								</TabsTrigger>
							);
						})}
					</TabsList>
				</ScrollArea>
				{tabChildren.map((child) => {
					const idx = getChildIdx(child);
					return (
						<TabsContent key={idx} value={idx}>
							{cloneElement(child, {
								isInGroup: true,
								...child.props,
							})}
						</TabsContent>
					);
				})}
			</Tabs>
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
	flush,
}: PreProps) => {
	const codeBlock = (
		<div className={cn(
			"not-prose group-[.code-group]:my-0 group-[.code-group]:-mt-2 group-[.code-group]:border-none group-[.code-group]:overflow-visible",
			flush ? "" : "my-4 overflow-hidden rounded-md border"
		)}>
			<div className="bg-neutral-950 text-wrap p-2 text-sm">
				<ScrollArea className="w-full">
					{children
						? cloneElement(children, { escaped: true })
						: null}
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
				<WithTooltip
					trigger={
						<CopyCodeTrigger>
							<Button size="icon-sm" variant="ghost">
								<Icon icon={faCopy} />
							</Button>
						</CopyCodeTrigger>
					}
					content="Copy code"
				/>
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

export const code = ({ children, escaped }) => {
	if (escaped) {
		return (
			<span
				className="not-prose code"
				// biome-ignore lint/security/noDangerouslySetInnerHtml: it's generated from markdown
				dangerouslySetInnerHTML={{ __html: children }}
			/>
		);
	}
	return (
		<code
			// biome-ignore lint/security/noDangerouslySetInnerHtml: it's generated from markdown
			dangerouslySetInnerHTML={{ __html: escapeHTML(children) }}
		/>
	);
};
