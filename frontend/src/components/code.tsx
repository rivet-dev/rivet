import { faCopy, faFile, Icon, type IconProp } from "@rivet-gg/icons";
import {
	Children,
	cloneElement,
	createContext,
	type ReactElement,
	type ReactNode,
	useCallback,
	useContext,
	useState,
} from "react";
import { CopyTrigger } from "./copy-area";
import { cn } from "./lib/utils";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";
import { ScrollArea } from "./ui/scroll-area";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./ui/tabs";

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

interface CodeGroupSyncContextValue {
	values: Record<string, string>;
	setValue: (syncId: string, value: string) => void;
}

const CodeGroupSyncContext = createContext<CodeGroupSyncContextValue | null>(
	null,
);

export function CodeGroupSyncProvider({ children }: { children: ReactNode }) {
	const [values, setValues] = useState<Record<string, string>>({});

	const setValue = useCallback((syncId: string, value: string) => {
		setValues((prev) => ({ ...prev, [syncId]: value }));
	}, []);

	return (
		<CodeGroupSyncContext.Provider value={{ values, setValue }}>
			{children}
		</CodeGroupSyncContext.Provider>
	);
}

export type CodeFrameLikeElement = ReactElement<{
	language?: keyof typeof languageNames;
	title?: string;
	icon?: IconProp;
	isInGroup?: boolean;
	file?: string;
}>;

interface CodeGroupProps {
	className?: string;
	header?: ReactNode;
	children: CodeFrameLikeElement[];
	syncId?: string;
}

const getChildIdx = (child: CodeGroupProps["children"][number]) =>
	child.props?.file || child.props?.title || child.props?.language || "code";

export function CodeGroup({
	children,
	className,
	syncId,
	header,
}: CodeGroupProps) {
	const syncContext = useContext(CodeGroupSyncContext);
	const defaultValue = getChildIdx(children[0]);

	const isControlled = syncId && syncContext;
	const value = isControlled
		? (syncContext.values[syncId] ?? defaultValue)
		: undefined;

	const handleValueChange = (newValue: string) => {
		if (isControlled) {
			syncContext.setValue(syncId, newValue);
		}
	};

	return (
		<div
			className={cn(
				"code-group group my-4 rounded-lg border pt-2",
				className,
			)}
		>
			{header}
			<Tabs
				defaultValue={!isControlled ? defaultValue : undefined}
				value={value}
				onValueChange={isControlled ? handleValueChange : undefined}
			>
				<ScrollArea
					className="w-full"
					viewportProps={{ className: "[&>div]:!table" }}
				>
					<TabsList className="pt-2">
						{Children.map(children, (child) => {
							const idx = getChildIdx(child);
							return (
								<TabsTrigger
									key={idx}
									value={idx}
									className="data-[state=active]:!text-white"
								>
									{child.props.icon ? (
										<>
											<Icon
												icon={child.props.icon}
												className="mr-1.5"
											/>
											{child.props.title ||
												languageNames[
													child.props.language ||
														"bash"
												] ||
												"Code"}
										</>
									) : (
										child.props.title ||
										languageNames[
											child.props.language || "bash"
										] ||
										"Code"
									)}
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
	icon?: IconProp;
	language: keyof typeof languageNames;
	isInGroup?: boolean;
	code?: () => string;
	footer?: ReactNode;
	children?: ReactElement<any>;
	className?: string;
}
export const CodeFrame = ({
	children,
	file,
	language,
	title,
	code,
	footer,
	isInGroup,
	className,
}: CodeFrameProps) => {
	return (
		<div
			className={cn(
				"not-prose my-4 overflow-hidden rounded-lg border group-[.code-group]:my-0 group-[.code-group]:-mt-2 group-[.code-group]:border-none",
				className,
			)}
		>
			<div className="bg-background text-wrap text-sm">
				<ScrollArea className="w-full px-1 py-4 [&_.shiki_.line:first-child]:max-w-">
					<CopyTrigger value={code || ""}>
						<Button
							variant="ghost"
							size="icon"
							className="absolute top-1.5 right-1.5 opacity-0 transition-opacity hover:opacity-100 group-hover:opacity-100"
						>
							<Icon icon={faCopy} />
						</Button>
					</CopyTrigger>
					{children
						? cloneElement(children, { escaped: true })
						: null}
				</ScrollArea>
			</div>

			{footer || file || !isInGroup ? (
				<div className="text-foreground flex items-center justify-between gap-2 border-t p-2 text-xs">
					<div className="text-muted-foreground flex items-center gap-1">
						{footer}
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
				</div>
			) : null}
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
