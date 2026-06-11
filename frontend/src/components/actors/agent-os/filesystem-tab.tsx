import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import {
	RelativeTime,
	ResizableHandle,
	ResizablePanel,
	ResizablePanelGroup,
	ScrollArea,
} from "@/components";
import type { ActorId } from "../queries";
import { AgentOsEmpty, formatBytes } from "./common";
import { FileTreeItem } from "./file-tree-item";
import { DEFAULT_FILE_PATH } from "./fixtures";
import type { FileContent, FsNode } from "./types";
import { useAgentOsInspector } from "./use-agent-os-inspector";

export function FilesystemTab({
	tree,
	selectedPath,
	content,
	onSelect,
}: {
	tree: FsNode;
	selectedPath: string | null;
	content: FileContent | null;
	onSelect: (node: FsNode) => void;
}) {
	const roots = tree.children ?? [];
	return (
		<ResizablePanelGroup direction="horizontal" className="h-full">
			<ResizablePanel defaultSize={34} minSize={22}>
				<div className="flex h-full flex-col">
					<div className="border-b px-3 py-2 font-mono text-xs text-muted-foreground">
						/
					</div>
					<ScrollArea className="min-h-0 flex-1 p-2">
						{roots.length === 0 ? (
							<AgentOsEmpty>Empty filesystem.</AgentOsEmpty>
						) : (
							roots.map((node) => (
								<FileTreeItem
									key={node.path}
									node={node}
									depth={0}
									selectedPath={selectedPath}
									onSelect={onSelect}
								/>
							))
						)}
					</ScrollArea>
				</div>
			</ResizablePanel>
			<ResizableHandle />
			<ResizablePanel defaultSize={66} minSize={30}>
				<FileViewer content={content} />
			</ResizablePanel>
		</ResizablePanelGroup>
	);
}

function FileViewer({ content }: { content: FileContent | null }) {
	if (!content) {
		return <AgentOsEmpty>Select a file to view its contents.</AgentOsEmpty>;
	}
	return (
		<div className="flex h-full flex-col">
			<div className="flex items-center gap-3 border-b px-4 py-3">
				<span className="truncate font-mono text-sm">
					{content.path}
				</span>
				<span className="ml-auto shrink-0 text-xs text-muted-foreground">
					{formatBytes(content.sizeBytes)}
				</span>
				<span className="shrink-0 text-xs text-muted-foreground">
					<RelativeTime time={new Date(content.mtimeMs)} />
				</span>
			</div>
			<ScrollArea className="min-h-0 flex-1">
				{content.text === null ? (
					<div className="flex h-full items-center justify-center p-8 text-center text-sm text-muted-foreground">
						Binary file ({formatBytes(content.sizeBytes)}) — preview
						unavailable.
					</div>
				) : (
					<pre className="whitespace-pre-wrap break-words p-4 font-mono text-xs leading-relaxed">
						{content.text}
					</pre>
				)}
			</ScrollArea>
		</div>
	);
}

export function FilesystemTabConnected({ actorId }: { actorId: ActorId }) {
	const inspector = useAgentOsInspector();
	const [selectedPath, setSelectedPath] = useState<string | null>(
		DEFAULT_FILE_PATH,
	);
	const { data: tree } = useQuery(inspector.filesystemQueryOptions(actorId));
	const { data: content = null } = useQuery({
		...inspector.fileContentQueryOptions(actorId, selectedPath ?? ""),
		enabled: !!selectedPath,
	});

	if (!tree) return <AgentOsEmpty>Loading filesystem…</AgentOsEmpty>;

	return (
		<FilesystemTab
			tree={tree}
			selectedPath={selectedPath}
			content={selectedPath ? content : null}
			onSelect={(node) => {
				if (node.type === "file") setSelectedPath(node.path);
			}}
		/>
	);
}
