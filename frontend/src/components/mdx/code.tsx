import { faCopy, faFile, Icon } from "@rivet-gg/icons";
import escapeHTML from "escape-html";
import {
	Children,
	cloneElement,
	type ReactElement,
	type ReactNode,
} from "react";
import { cn } from "../lib/utils";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import { ScrollArea } from "../ui/scroll-area";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "../ui/tabs";
import { WithTooltip } from "../ui/tooltip";
import { CopyCodeTrigger } from "./code-buttons";

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
			data-code-group
		>
			<Tabs defaultValue={getChildIdx(children[0])}>
				<div className="flex gap-1 border-b pr-2">
					<ScrollArea
						className="w-full"
						viewportProps={{ className: "[&>div]:!table" }}
					>
						<TabsList className="border-b-0">
							{Children.map(children, (child) => {
								const idx = getChildIdx(child);
								return (
									<TabsTrigger key={idx} value={idx}>
										{child.props.title ||
											languageNames[
												child.props.language
											] ||
											"Code"}
									</TabsTrigger>
								);
							})}
						</TabsList>
					</ScrollArea>
					<WithTooltip
						trigger={
							<CopyCodeTrigger>
								<Button
									size="icon-sm"
									className="text-foreground"
									variant="ghost"
								>
									<Icon icon={faCopy} />
								</Button>
							</CopyCodeTrigger>
						}
						content="Copy code"
					/>
				</div>
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
	language?: keyof typeof languageNames;
	isInGroup?: boolean;
	children?: ReactNode;
}
export const pre = ({
	children,
	file,
	language,
	title,
	isInGroup,
}: PreProps) => {
	return (
		<div
			className="not-prose my-4 rounded-md border group-[.code-group]:my-0 group-[.code-group]:-mt-2 group-[.code-group]:border-none"
			data-code-group
		>
			{!file && isInGroup ? null : (
				<div className="text-foreground flex items-center justify-between gap-2 border-b p-2 text-xs">
					<div className="text-muted-foreground flex items-center gap-1">
						{file ? (
							<Badge variant="outline">
								<Icon icon={faFile} className="mr-1" />
								<span>{file}</span>
							</Badge>
						) : isInGroup ? null : (
							<Badge variant="outline">
								{title || languageNames[language || "cpp"]}
							</Badge>
						)}
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
			)}
			<div className="bg-background text-wrap p-2 text-sm">
				<ScrollArea className="w-full">
					{children
						? cloneElement(children as ReactElement, {
								escaped: true,
							})
						: null}
				</ScrollArea>
			</div>
		</div>
	);
};

export { pre as Code };

export const code = ({
	children,
	escaped,
}: {
	children?: ReactNode;
	escaped?: boolean;
}) => {
	if (escaped) {
		return (
			<span
				className="not-prose code"
				// biome-ignore lint/security/noDangerouslySetInnerHtml: it's generated from markdown
				dangerouslySetInnerHTML={{ __html: children as string }}
			/>
		);
	}
	return (
		<code
			// biome-ignore lint/security/noDangerouslySetInnerHtml: it's generated from markdown
			dangerouslySetInnerHTML={{ __html: escapeHTML(children as string) }}
		/>
	);
};
