import { javascript } from "@codemirror/lang-javascript";
import { json, jsonParseLinter } from "@codemirror/lang-json";
import { linter } from "@codemirror/lint";
import { EditorView } from "@codemirror/view";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@radix-ui/react-tabs";
import { Icon, faCopy, faFile } from "@rivet-gg/icons";
import { githubDark } from "@uiw/codemirror-theme-github";
import ReactCodeMirror, {
	type ReactCodeMirrorProps,
} from "@uiw/react-codemirror";
import { Children, type ReactElement, cloneElement, forwardRef } from "react";
import { CodeMirrorContainer } from "./code-mirror-container";
import { CopyButton } from "./copy-area";
import { cn } from "./lib/utils";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";
import { ScrollArea } from "./ui/scroll-area";
import { WithTooltip } from "./ui/tooltip";

interface JsonCodeProps extends ReactCodeMirrorProps {}

export const JsonCode = forwardRef<HTMLDivElement, JsonCodeProps>(
	({ value, extensions = [], ...props }, ref) => {
		return (
			<CodeMirrorContainer ref={ref} tabIndex={0}>
				<ReactCodeMirror
					{...props}
					extensions={[
						json(),
						linter(jsonParseLinter()),
						...extensions,
					]}
					theme={githubDark}
					value={value}
				/>
			</CodeMirrorContainer>
		);
	},
);

export const JavaScriptCode = forwardRef<HTMLDivElement, JsonCodeProps>(
	({ value, extensions = [], ...props }, ref) => {
		return (
			<CodeMirrorContainer ref={ref}>
				<ReactCodeMirror
					{...props}
					basicSetup={{}}
					extensions={[
						javascript(),
						EditorView.lineWrapping,
						...extensions,
					]}
					theme={githubDark}
					value={value}
				/>
			</CodeMirrorContainer>
		);
	},
);

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
		<div className="not-prose my-4 rounded-md border group-[.code-group]:my-0 group-[.code-group]:-mt-2 group-[.code-group]:border-none">
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
