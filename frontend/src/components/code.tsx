import { faCopy, faFile, Icon } from "@rivet-gg/icons";
import { Children, cloneElement, type ReactElement } from "react";
import { CopyButton } from "./copy-area";
import { cn } from "./lib/utils";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";
import { ScrollArea } from "./ui/scroll-area";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./ui/tabs";
import { WithTooltip } from "./ui/tooltip";

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
};

interface CodeGroupProps {
	className?: string;
	children: ReactElement<{
		language?: keyof typeof languageNames;
		title?: string;
		isInGroup?: boolean;
	}>[];
}

const getChildIdx = (child: ReactElement) =>
	child.props?.file || child.props?.title || child.props?.language || "code";

export function CodeGroup({ children, className }: CodeGroupProps) {
	return (
		<div
			className={cn(
				"code-group group my-4 rounded-lg border pt-2",
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
										languageNames[
											child.props.language || "bash"
										] ||
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

interface CodeFrameProps {
	file?: string;
	title?: string;
	language: keyof typeof languageNames;
	isInGroup?: boolean;
	code?: string;
	children?: ReactElement;
}
export const CodeFrame = ({
	children,
	file,
	language,
	code,
	title,
	isInGroup,
}: CodeFrameProps) => {
	return (
		<div className="not-prose my-4 rounded-lg border group-[.code-group]:my-0 group-[.code-group]:-mt-2 group-[.code-group]:border-none">
			<div className="bg-background text-wrap p-2 text-sm">
				<ScrollArea className="w-full">
					{children
						? cloneElement(children, { escaped: true })
						: null}
				</ScrollArea>
			</div>

			<div className="text-foreground flex items-center justify-between gap-2 border-t p-2 text-xs">
				<div className="text-muted-foreground flex items-center gap-1">
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
				</div>
				<WithTooltip
					trigger={
						<CopyButton value={code || ""}>
							<Button size="icon-sm" variant="ghost">
								<Icon icon={faCopy} />
							</Button>
						</CopyButton>
					}
					content="Copy code"
				/>
			</div>
		</div>
	);
};

interface CodeSoruceProps {
	children: string;
	escaped?: boolean;
}
export const CodeSource = ({ children, escaped }: CodeSoruceProps) => {
	if (escaped) {
		return (
			<span
				className="not-prose code"
				/* biome-ignore lint/security/noDangerouslySetInnerHtml: its safe bc we generate that code */
				dangerouslySetInnerHTML={{ __html: children }}
			/>
		);
	}
	return (
		<code
			// TODO: add escapeHTML
			/* biome-ignore lint/security/noDangerouslySetInnerHtml: its safe bc we generate that code */
			dangerouslySetInnerHTML={{ __html: children }}
		/>
	);
};
