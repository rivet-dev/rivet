import { faChevronRight, faFile, faFolder, Icon } from "@rivet-gg/icons";
import { useState } from "react";
import { cn } from "@/components";
import {
	Collapsible,
	CollapsibleContent,
	CollapsibleTrigger,
} from "@/components/ui/collapsible";
import type { FsNode } from "./types";

interface FileTreeItemProps {
	node: FsNode;
	depth: number;
	selectedPath: string | null;
	onSelect: (node: FsNode) => void;
}

export function FileTreeItem({
	node,
	depth,
	selectedPath,
	onSelect,
}: FileTreeItemProps) {
	const isDir = node.type === "dir";
	const hasChildren = isDir && (node.children?.length ?? 0) > 0;
	const [open, setOpen] = useState(depth < 2);
	const isSelected = node.path === selectedPath;

	return (
		<Collapsible open={open} onOpenChange={setOpen}>
			<div
				className="flex items-center gap-1"
				style={{ paddingLeft: `${depth * 12}px` }}
			>
				<CollapsibleTrigger
					className={cn(
						"flex size-5 items-center justify-center rounded transition-colors hover:bg-accent",
						!hasChildren && "pointer-events-none opacity-0",
					)}
					disabled={!hasChildren}
				>
					<Icon
						icon={faChevronRight}
						className={cn(
							"size-3 text-muted-foreground transition-transform",
							open && "rotate-90",
						)}
					/>
				</CollapsibleTrigger>
				<button
					type="button"
					onClick={() =>
						isDir ? setOpen((v) => !v) : onSelect(node)
					}
					className={cn(
						"flex min-w-0 flex-1 items-center gap-2 rounded px-2 py-1 text-left text-sm transition-colors",
						isSelected
							? "bg-accent text-accent-foreground"
							: "hover:bg-accent/50",
					)}
				>
					<Icon
						icon={isDir ? faFolder : faFile}
						className="size-3.5 shrink-0 text-muted-foreground"
					/>
					<span className="flex-1 truncate">{node.name}</span>
					<span className="shrink-0 text-[10px] uppercase tracking-wide text-muted-foreground/60">
						{node.mount}
					</span>
				</button>
			</div>
			{hasChildren ? (
				<CollapsibleContent>
					{node.children?.map((child) => (
						<FileTreeItem
							key={child.path}
							node={child}
							depth={depth + 1}
							selectedPath={selectedPath}
							onSelect={onSelect}
						/>
					))}
				</CollapsibleContent>
			) : null}
		</Collapsible>
	);
}
