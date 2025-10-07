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
import { Children, cloneElement, type ReactElement } from "react";
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
	yaml: "YAML",
	gdscript: "GDScript",
	powershell: "Command Line",
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
	child.props?.file || child.props?.title || child.props?.language || "code";

export function CodeGroup({ children, className }: CodeGroupProps) {
	return (
		<div
			className={cn(
				"code-group group my-4 rounded-md border pt-2",
				className,
			)}
		>
			<Tabs defaultValue={getChildIdx(children[0])}>
				<ScrollArea
					className="w-full"
					viewportProps={{ className: "[&>div]:!table" }}
				>
					<TabsList>
						{Children.map(children, (child) => {
							const idx = getChildIdx(child);
							return (
								<TabsTrigger key={idx} value={idx}>
									{child.props.title ||
										languageNames[child.props.language] ||
										"Code"}
								</TabsTrigger>
							);
						})}
					</TabsList>
				</ScrollArea>
				{Children.map(children, (child) => {
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
	language: keyof typeof languageNames;
	isInGroup?: boolean;
	children?: ReactElement;
	autofill?: boolean;
	code?: string;
}
export const pre = ({
	children,
	file,
	language,
	title,
	isInGroup,
	autofill,
	code,
}: PreProps) => {
	const codeBlock = (
		<div className="not-prose my-4 rounded-md border group-[.code-group]:my-0 group-[.code-group]:-mt-2 group-[.code-group]:border-none">
			<div className="bg-background text-wrap p-2 text-sm">
				<ScrollArea className="w-full">
					{children
						? cloneElement(children, { escaped: true })
						: null}
				</ScrollArea>
			</div>

			<div className="text-foreground flex items-center justify-between gap-2 border-t p-2 text-xs">
				<div className="text-muted-foreground flex items-center gap-2">
					{file ? (
						<>
							<Icon icon={faFile} className="block" />
							<span>{file}</span>
						</>
					) : isInGroup ? null : (
						<Badge variant="outline">
							{title || languageNames[language]}
						</Badge>
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
